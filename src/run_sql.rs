//! Built-in `run-sql` action resolution and execution.

use crate::database::{connect_postgres_client, resolve_database_dsn};
use crate::findings::FindingsPayload;
use crate::report_store::ReportStore;
use crate::triage::{ActionKind, NextActionStatus, OperatingMode, PgTriageReport};
use crate::{
    sql_action_report, workflow_slug, AppConfig, PgLogstatsError, Result, SqlActionPayload,
};
use postgres::types::ToSql;
use std::collections::BTreeMap;
use std::path::Path;

const MAX_SQL_ROWS: usize = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionParameterInput {
    /// Parameter name defined by the selected built-in action.
    pub name: String,
    /// Caller-provided value for the named parameter.
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct RunSqlRequest<'a> {
    /// Workspace used to load persisted triage reports.
    pub workspace_path: &'a Path,
    /// Parent triage report that exposed the selected action.
    pub triage_report: &'a str,
    /// Selected built-in action identifier.
    pub action_id: &'a str,
    /// Optional DSN override for database access.
    pub dsn: Option<&'a str>,
    /// Operating mode already established by startup inspect loading.
    pub operating_mode: OperatingMode,
    /// Caller-provided parameter values for the selected action.
    pub parameters: &'a [ActionParameterInput],
}

#[derive(Debug)]
#[allow(dead_code)]
struct SelectedQueryFamilyAction {
    report: PgTriageReport<FindingsPayload>,
    action: crate::NextAction,
    finding: crate::Finding,
}

#[derive(Debug, Clone)]
enum BoundParameter {
    Int8(i64),
    Text(String),
}

impl BoundParameter {
    fn boxed(self) -> Box<dyn ToSql + Sync> {
        match self {
            Self::Int8(value) => Box::new(value),
            Self::Text(value) => Box::new(value),
        }
    }
}

#[derive(Debug)]
struct PreparedQuery {
    sql: String,
    parameters: Vec<BoundParameter>,
}

pub fn parse_action_parameters(raw: &[String]) -> Result<Vec<ActionParameterInput>> {
    let mut params = Vec::new();
    for item in raw {
        let (name, value) = item
            .split_once('=')
            .ok_or_else(|| PgLogstatsError::Configuration {
                message: format!("Invalid --parameter '{}'. Expected NAME=VALUE.", item),
                field: Some("parameter".to_string()),
            })?;

        let name = name.trim();
        if name.is_empty() {
            return Err(PgLogstatsError::Configuration {
                message: format!("Invalid --parameter '{}'. Parameter name is empty.", item),
                field: Some("parameter".to_string()),
            });
        }

        params.push(ActionParameterInput {
            name: name.to_string(),
            value: value.to_string(),
        });
    }

    Ok(params)
}

fn prepare_running_query_sql(
    rule_id: &str,
    action: &crate::NextAction,
    raw_parameters: &[ActionParameterInput],
) -> Result<PreparedQuery> {
    let parameters = parameter_map(raw_parameters)?;
    match rule_id {
        "running_query.pg_stat_activity.by_pid" => {
            let pid = resolve_i64_parameter("pid", action.target.as_deref(), &parameters)?
                .ok_or_else(|| PgLogstatsError::Configuration {
                    message: format!("Action '{}' requires pid", action.action_id),
                    field: Some("selected_action_id".to_string()),
                })?;

            Ok(PreparedQuery {
                sql: "SELECT pid, usename, datname, application_name, client_addr, backend_start, xact_start, query_start, state_change, wait_event_type, wait_event, state, query_id, query FROM pg_stat_activity WHERE pid = $1;".to_string(),
                parameters: vec![BoundParameter::Int8(pid)],
            })
        }
        "running_query.blocking.by_pid" => {
            let pid = resolve_i64_parameter("pid", action.target.as_deref(), &parameters)?
                .ok_or_else(|| PgLogstatsError::Configuration {
                    message: format!("Action '{}' requires pid", action.action_id),
                    field: Some("selected_action_id".to_string()),
                })?;

            Ok(PreparedQuery {
                sql: "SELECT pid, usename, datname, application_name, state, wait_event_type, wait_event, query FROM pg_stat_activity WHERE pid = ANY(pg_blocking_pids($1));".to_string(),
                parameters: vec![BoundParameter::Int8(pid)],
            })
        }
        "query_family.pg_stat_statements.by_queryid" => {
            let queryid = resolve_i64_parameter("queryid", action.target.as_deref(), &parameters)?
                .ok_or_else(|| PgLogstatsError::Configuration {
                    message: format!("Action '{}' requires queryid", action.action_id),
                    field: Some("selected_action_id".to_string()),
                })?;

            Ok(PreparedQuery {
                sql: "SELECT queryid, calls, total_exec_time, mean_exec_time, min_exec_time, max_exec_time, rows, shared_blks_hit, shared_blks_read, temp_blks_read, temp_blks_written, query FROM pg_stat_statements WHERE queryid = $1;".to_string(),
                parameters: vec![BoundParameter::Int8(queryid)],
            })
        }
        _ => Err(PgLogstatsError::Configuration {
            message: format!(
                "run-sql does not support selected action '{}'",
                action.action_id
            ),
            field: Some("selected_action_id".to_string()),
        }),
    }
}

/// Resolve a selected built-in SQL action from the parent report, bind caller
/// parameters, execute it against PostgreSQL, and return a `run_sql` report.
pub fn execute_run_sql(
    request: &RunSqlRequest<'_>,
    config: &AppConfig,
) -> Result<PgTriageReport<SqlActionPayload>> {
    let selected_action_id = request.action_id;
    let store = ReportStore::new(request.workspace_path);
    let base_report = store.load_report_base(request.triage_report)?;
    let parent_content = base_report.raw_content.clone();

    let action = base_report
        .next_actions
        .iter()
        .find(|a| a.action_id == selected_action_id)
        .cloned()
        .ok_or_else(|| PgLogstatsError::Configuration {
            message: format!(
                "Action ID '{}' not found in parent report next_actions",
                selected_action_id
            ),
            field: Some("action_id".to_string()),
        })?;

    if action.status != NextActionStatus::Allowed {
        return Err(PgLogstatsError::Configuration {
            message: format!(
                "Action '{}' is not allowed in parent report. Status: {:?}, Reason: {}",
                selected_action_id, action.status, action.reason
            ),
            field: Some("action_id".to_string()),
        });
    }

    if action.kind != ActionKind::RunSql {
        return Err(PgLogstatsError::Configuration {
            message: format!(
                "Action '{}' is not a SQL action. Use the action according to its type instead of `pg-logstats run-sql`.",
                selected_action_id
            ),
            field: Some("action_id".to_string()),
        });
    }

    let prepared = match base_report.workflow {
        ActionKind::TopQueryFamilies | ActionKind::Errors | ActionKind::TempFiles => {
            let report: PgTriageReport<FindingsPayload> =
                serde_json::from_str(&parent_content).map_err(PgLogstatsError::Serialization)?;
            let finding_id =
                action
                    .target
                    .as_deref()
                    .ok_or_else(|| PgLogstatsError::Configuration {
                        message: format!(
                            "Action '{}' does not include a finding target",
                            selected_action_id
                        ),
                        field: Some("action_id".to_string()),
                    })?;
            let finding = report
                .payload
                .findings
                .iter()
                .find(|f| f.target_id() == finding_id)
                .cloned()
                .ok_or_else(|| PgLogstatsError::Configuration {
                    message: format!(
                        "Action '{}' refers to missing finding '{}'",
                        selected_action_id, finding_id
                    ),
                    field: Some("action_id".to_string()),
                })?;
            prepare_findings_workflow_sql(&action, &finding, request.parameters)?
        }
        ActionKind::RunningQueries => {
            let rule_id = selected_action_id.split(':').next().ok_or_else(|| {
                PgLogstatsError::Configuration {
                    message: format!("Invalid action id '{}'", selected_action_id),
                    field: Some("action_id".to_string()),
                }
            })?;
            prepare_running_query_sql(rule_id, &action, request.parameters)?
        }
        workflow => {
            return Err(PgLogstatsError::Configuration {
                message: format!(
                    "run-sql does not support parent workflow '{}' yet",
                    workflow_slug(workflow)
                ),
                field: Some("action_id".to_string()),
            })
        }
    };

    let resolved_dsn =
        resolve_database_dsn(request.dsn, config).ok_or_else(|| PgLogstatsError::Configuration {
            message: "Database connection not configured. Specify --dsn, PG_LOGSTATS_DATABASE_URL, or [database].dsn in config.".to_string(),
            field: Some("database_connection_not_configured".to_string()),
        })?;

    let mut client = connect_postgres_client(&resolved_dsn, config.database.connect_timeout_ms)
        .map_err(|e| PgLogstatsError::Configuration {
            message: e,
            field: Some("dsn".to_string()),
        })?;

    let boxed_params: Vec<Box<dyn ToSql + Sync>> = prepared
        .parameters
        .into_iter()
        .map(BoundParameter::boxed)
        .collect();
    let param_refs: Vec<&(dyn ToSql + Sync)> = boxed_params
        .iter()
        .map(|param| param.as_ref() as &(dyn ToSql + Sync))
        .collect();

    let rows =
        client
            .query(&prepared.sql, &param_refs)
            .map_err(|e| PgLogstatsError::Unexpected {
                message: format!("Failed to execute SQL: {}", e),
                context: None,
            })?;

    let mut columns = Vec::new();
    if !rows.is_empty() {
        for col in rows[0].columns() {
            columns.push(col.name().to_string());
        }
    }

    use postgres::types::Type;
    let mut json_rows = Vec::new();
    for row in rows {
        let mut json_row = Vec::new();
        for (i, col) in row.columns().iter().enumerate() {
            let val: serde_json::Value = match col.type_() {
                &Type::INT4 => {
                    let v: Option<i32> = row.get(i);
                    v.map(Into::into).unwrap_or(serde_json::Value::Null)
                }
                &Type::INT8 => {
                    let v: Option<i64> = row.get(i);
                    v.map(Into::into).unwrap_or(serde_json::Value::Null)
                }
                &Type::FLOAT4 => {
                    let v: Option<f32> = row.get(i);
                    v.map(Into::into).unwrap_or(serde_json::Value::Null)
                }
                &Type::FLOAT8 => {
                    let v: Option<f64> = row.get(i);
                    v.map(Into::into).unwrap_or(serde_json::Value::Null)
                }
                &Type::VARCHAR | &Type::TEXT | &Type::NAME => {
                    let v: Option<String> = row.get(i);
                    v.map(Into::into).unwrap_or(serde_json::Value::Null)
                }
                &Type::BOOL => {
                    let v: Option<bool> = row.get(i);
                    v.map(Into::into).unwrap_or(serde_json::Value::Null)
                }
                _ => {
                    let v: std::result::Result<String, _> = row.try_get(i);
                    v.map(Into::into).unwrap_or(serde_json::Value::Null)
                }
            };
            json_row.push(val);
        }
        json_rows.push(json_row);
    }

    let row_count = json_rows.len();
    let truncated = row_count > MAX_SQL_ROWS;
    if truncated {
        json_rows.truncate(MAX_SQL_ROWS);
    }

    let payload = SqlActionPayload {
        action_id: action.action_id.clone(),
        source_report_id: base_report.report_id.clone(),
        sql: prepared.sql,
        row_count,
        truncated,
        columns,
        rows: json_rows,
    };

    Ok(sql_action_report(payload, request.operating_mode))
}

fn prepare_findings_workflow_sql(
    action: &crate::NextAction,
    finding: &crate::Finding,
    raw_parameters: &[ActionParameterInput],
) -> Result<PreparedQuery> {
    let parameters = parameter_map(raw_parameters)?;
    let rule_id =
        action
            .action_id
            .split(':')
            .next()
            .ok_or_else(|| PgLogstatsError::Configuration {
                message: format!("Invalid action id '{}'", action.action_id),
                field: Some("selected_action_id".to_string()),
            })?;

    match rule_id {
        // QueryFamily rules
        "query_family.pg_stat_statements.by_queryid" => {
            let query_family = finding.query_family.as_ref().ok_or_else(|| PgLogstatsError::Configuration {
                message: format!("Action '{}' requires a query_family finding target", action.action_id),
                field: Some("selected_action_id".to_string()),
            })?;
            let queryid =
                resolve_i64_parameter("queryid", query_family.queryid.as_deref(), &parameters)?
                    .ok_or_else(|| PgLogstatsError::Configuration {
                        message: format!("Action '{}' requires queryid", action.action_id),
                        field: Some("selected_action_id".to_string()),
                    })?;

            Ok(PreparedQuery {
                sql: "SELECT queryid, calls, total_exec_time, mean_exec_time, min_exec_time, max_exec_time, rows, shared_blks_hit, shared_blks_read, temp_blks_read, temp_blks_written, query FROM pg_stat_statements WHERE queryid = $1;".to_string(),
                parameters: vec![BoundParameter::Int8(queryid)],
            })
        }
        "query_family.pg_stat_activity.by_dimensions" => {
            let query_family = finding.query_family.as_ref().ok_or_else(|| PgLogstatsError::Configuration {
                message: format!("Action '{}' requires a query_family finding target", action.action_id),
                field: Some("selected_action_id".to_string()),
            })?;
            prepare_dimensions_activity_sql(
                action,
                query_family.database.as_deref(),
                query_family.user.as_deref(),
                query_family.application_name.as_deref(),
                &parameters,
            )
        }
        // ErrorClass rules
        "error_class.pg_stat_activity.by_dimensions" => {
            let error_class = finding.error_class.as_ref().ok_or_else(|| PgLogstatsError::Configuration {
                message: format!("Action '{}' requires an error_class finding target", action.action_id),
                field: Some("selected_action_id".to_string()),
            })?;
            prepare_dimensions_activity_sql(
                action,
                error_class.database.as_deref(),
                error_class.user.as_deref(),
                error_class.application_name.as_deref(),
                &parameters,
            )
        }
        // TempFile rules
        "temp_file.pg_stat_database.temp_counters" => {
            let temp_file = finding.temp_file.as_ref().ok_or_else(|| PgLogstatsError::Configuration {
                message: format!("Action '{}' requires a temp_file finding target", action.action_id),
                field: Some("selected_action_id".to_string()),
            })?;
            let database =
                resolve_text_parameter("database", temp_file.database.as_deref(), &parameters)?
                    .ok_or_else(|| PgLogstatsError::Configuration {
                        message: format!("Action '{}' requires database", action.action_id),
                        field: Some("selected_action_id".to_string()),
                    })?;
            Ok(PreparedQuery {
                sql: "SELECT datname, temp_files, temp_bytes FROM pg_stat_database WHERE datname = $1;".to_string(),
                parameters: vec![BoundParameter::Text(database)],
            })
        }
        "temp_file.pg_stat_statements.temp_blocks" => {
            Ok(PreparedQuery {
                sql: "SELECT queryid, calls, total_exec_time, temp_blks_read, temp_blks_written, query FROM pg_stat_statements WHERE temp_blks_read > 0 OR temp_blks_written > 0 ORDER BY temp_blks_read + temp_blks_written DESC LIMIT 20;".to_string(),
                parameters: vec![],
            })
        }
        _ => Err(PgLogstatsError::Configuration {
            message: format!(
                "run-sql does not support selected action '{}'",
                action.action_id
            ),
            field: Some("selected_action_id".to_string()),
        }),
    }
}

fn prepare_dimensions_activity_sql(
    action: &crate::NextAction,
    db_val: Option<&str>,
    user_val: Option<&str>,
    app_val: Option<&str>,
    parameters: &BTreeMap<String, String>,
) -> Result<PreparedQuery> {
    let database = resolve_text_parameter("database", db_val, parameters)?;
    let user = resolve_text_parameter("user", user_val, parameters)?;
    let application_name = resolve_text_parameter("application_name", app_val, parameters)?;

    let mut clauses = Vec::new();
    let mut bound = Vec::new();

    if let Some(database) = database {
        clauses.push(format!("datname = ${}", bound.len() + 1));
        bound.push(BoundParameter::Text(database));
    }
    if let Some(user) = user {
        clauses.push(format!("usename = ${}", bound.len() + 1));
        bound.push(BoundParameter::Text(user));
    }
    if let Some(application_name) = application_name {
        clauses.push(format!("application_name = ${}", bound.len() + 1));
        bound.push(BoundParameter::Text(application_name));
    }

    if clauses.is_empty() {
        return Err(PgLogstatsError::Configuration {
            message: format!(
                "Action '{}' requires at least one of database, user, or application_name",
                action.action_id
            ),
            field: Some("selected_action_id".to_string()),
        });
    }

    Ok(PreparedQuery {
        sql: format!(
            "SELECT pid, usename, datname, application_name, state, wait_event_type, wait_event, query_start, query_id, query FROM pg_stat_activity WHERE {} AND state <> 'idle' ORDER BY query_start DESC NULLS LAST LIMIT 20;",
            clauses.join(" AND ")
        ),
        parameters: bound,
    })
}

fn parameter_map(parameters: &[ActionParameterInput]) -> Result<BTreeMap<String, String>> {
    let mut values = BTreeMap::new();
    for parameter in parameters {
        if values
            .insert(parameter.name.clone(), parameter.value.clone())
            .is_some()
        {
            return Err(PgLogstatsError::Configuration {
                message: format!("Duplicate --parameter '{}'", parameter.name),
                field: Some("parameter".to_string()),
            });
        }
    }
    Ok(values)
}

fn resolve_text_parameter(
    name: &str,
    source_value: Option<&str>,
    parameters: &BTreeMap<String, String>,
) -> Result<Option<String>> {
    match (source_value, parameters.get(name)) {
        (Some(source_value), Some(parameter_value)) if source_value != parameter_value => {
            Err(PgLogstatsError::Configuration {
                message: format!(
                    "Parameter '{}' conflicts with the selected action context",
                    name
                ),
                field: Some("parameter".to_string()),
            })
        }
        (Some(source_value), _) => Ok(Some(source_value.to_string())),
        (None, Some(parameter_value)) => Ok(Some(parameter_value.clone())),
        (None, None) => Ok(None),
    }
}

fn resolve_i64_parameter(
    name: &str,
    source_value: Option<&str>,
    parameters: &BTreeMap<String, String>,
) -> Result<Option<i64>> {
    let source_value = source_value
        .map(|value| parse_i64_parameter(name, value))
        .transpose()?;
    let parameter_value = parameters
        .get(name)
        .map(|value| parse_i64_parameter(name, value))
        .transpose()?;

    match (source_value, parameter_value) {
        (Some(source_value), Some(parameter_value)) if source_value != parameter_value => {
            Err(PgLogstatsError::Configuration {
                message: format!(
                    "Parameter '{}' conflicts with the selected action context",
                    name
                ),
                field: Some("parameter".to_string()),
            })
        }
        (Some(source_value), _) => Ok(Some(source_value)),
        (None, Some(parameter_value)) => Ok(Some(parameter_value)),
        (None, None) => Ok(None),
    }
}

fn parse_i64_parameter(name: &str, value: &str) -> Result<i64> {
    value
        .parse::<i64>()
        .map_err(|_| PgLogstatsError::Configuration {
            message: format!("Parameter '{}' must be an integer", name),
            field: Some("parameter".to_string()),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::findings::{
        Finding, FindingConfidence, FindingKind, FindingMetrics, QueryFamilyFinding, ReasonCode,
    };
    use crate::triage::{NextAction, NextActionPriority, NextActionType};

    fn selected_query_family_action(action_id: &str) -> SelectedQueryFamilyAction {
        SelectedQueryFamilyAction {
            report: PgTriageReport {
                schema_version: 1,
                workflow: ActionKind::TopQueryFamilies,
                operating_mode: OperatingMode::LogBackedAndLive,
                limitations: Vec::new(),
                verdict: None,
                verdict_reasons: Vec::new(),
                allowed_actions: None,
                blocked_actions: None,
                analysis_window: None,
                source_summary: None,
                next_actions: Vec::new(),
                report_id: Some("0001-top_query_families".to_string()),
                parent_report_id: None,
                selected_action_id: None,
                created_at: None,
                payload: FindingsPayload {
                    findings: Vec::new(),
                },
            },
            action: NextAction {
                action_id: action_id.to_string(),
                action_type: NextActionType::RunSql,
                kind: ActionKind::RunSql,
                label: "test".to_string(),
                status: NextActionStatus::Allowed,
                priority: NextActionPriority::Recommended,
                judgement_required: true,
                reason: "test".to_string(),
                target: Some("demo".to_string()),
                workflow: Some(ActionKind::RunSql),
                command: None,
                survey: None,
                sql_preview: None,
                parameters: None,
                risk: None,
                action_class: None,
                required_identifiers: None,
                requires: None,
                produces: None,
            },
            finding: Finding {
                id: "demo".to_string(),
                kind: FindingKind::QueryFamily,
                rank: 1,
                title: "demo".to_string(),
                reason: "demo".to_string(),
                reason_codes: vec![ReasonCode::HighTotalDuration],
                score: 1.0,
                query_family: Some(QueryFamilyFinding {
                    query_family_id: "demo".to_string(),
                    normalized_sql: "SELECT 1".to_string(),
                    queryid: Some("918273645".to_string()),
                    database: Some("appdb".to_string()),
                    user: Some("app".to_string()),
                    application_name: Some("api".to_string()),
                    missing_attribution: Vec::new(),
                }),
                metrics: FindingMetrics {
                    execution_count: 1,
                    total_duration_ms: 1.0,
                    avg_duration_ms: 1.0,
                    max_duration_ms: 1.0,
                    correlated_execution_count: 1,
                    uncorrelated_execution_count: 0,
                },
                baseline: None,
                target: None,
                delta: None,
                evidence: Vec::new(),
                confidence: FindingConfidence::High,
                error_class: None,
                temp_file: None,
            },
        }
    }

    #[test]
    fn parses_action_parameters() {
        let parsed = parse_action_parameters(&["queryid=123".to_string()]).unwrap();
        assert_eq!(
            parsed,
            vec![ActionParameterInput {
                name: "queryid".to_string(),
                value: "123".to_string()
            }]
        );
    }

    #[test]
    fn queryid_sql_uses_bound_parameter() {
        let selected =
            selected_query_family_action("query_family.pg_stat_statements.by_queryid:demo");
        let prepared =
            prepare_findings_workflow_sql(&selected.action, &selected.finding, &[]).unwrap();

        assert_eq!(
            prepared.sql,
            "SELECT queryid, calls, total_exec_time, mean_exec_time, min_exec_time, max_exec_time, rows, shared_blks_hit, shared_blks_read, temp_blks_read, temp_blks_written, query FROM pg_stat_statements WHERE queryid = $1;"
        );
        assert!(matches!(
            prepared.parameters[0],
            BoundParameter::Int8(918273645)
        ));
    }

    #[test]
    fn activity_sql_binds_exact_dimensions() {
        let selected =
            selected_query_family_action("query_family.pg_stat_activity.by_dimensions:demo");
        let prepared =
            prepare_findings_workflow_sql(&selected.action, &selected.finding, &[]).unwrap();

        assert!(prepared.sql.contains("datname = $1"));
        assert!(prepared.sql.contains("usename = $2"));
        assert!(prepared.sql.contains("application_name = $3"));
        assert_eq!(prepared.parameters.len(), 3);
    }

    #[test]
    fn conflicting_parameter_override_is_rejected() {
        let selected =
            selected_query_family_action("query_family.pg_stat_activity.by_dimensions:demo");
        let err = prepare_findings_workflow_sql(
            &selected.action,
            &selected.finding,
            &[ActionParameterInput {
                name: "database".to_string(),
                value: "otherdb".to_string(),
            }],
        )
        .unwrap_err();

        assert!(err
            .to_string()
            .contains("Parameter 'database' conflicts with the selected action context"));
    }
}
