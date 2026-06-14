//! Built-in `run-sql` action resolution and execution.

use crate::database::{connect_postgres_client, resolve_database_dsn};
use crate::findings::FindingsPayload;
use crate::report_store::ReportStore;
use crate::triage::{
    ActionKind, NextActionStatus, NextActionType, OperatingMode, PgTriageReport, SqlActionInsight,
    SqlInsightConfidence,
};
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

#[derive(Debug, Default)]
struct ActivityInsightContext {
    row_count: usize,
    active_rows: usize,
    lock_wait_rows: usize,
    transactionid_lock_rows: usize,
    io_wait_rows: usize,
    client_wait_rows: usize,
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
            let pid = resolve_i64_parameter("pid", action.target_id.as_deref(), &parameters)?
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
            let pid = resolve_i64_parameter("pid", action.target_id.as_deref(), &parameters)?
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
            let queryid =
                resolve_i64_parameter("queryid", action.target_id.as_deref(), &parameters)?
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

    if action.action_type != NextActionType::RunSql {
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
                    .target_id
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
        source_finding_id: action.target_id.clone(),
        insights: derive_sql_action_insights(&action.action_id, &columns, &json_rows),
        row_count,
        truncated,
        columns,
        rows: json_rows,
    };

    Ok(sql_action_report(payload, request.operating_mode))
}

fn derive_sql_action_insights(
    action_id: &str,
    columns: &[String],
    rows: &[Vec<serde_json::Value>],
) -> Vec<SqlActionInsight> {
    let rule_id = action_id.split(':').next().unwrap_or(action_id);
    match rule_id {
        "query_family.pg_stat_activity.by_dimensions" => {
            derive_pg_stat_activity_dimension_insights(rows, columns)
        }
        "temp_file.pg_stat_database.temp_counters" => {
            derive_temp_file_database_insights(rows, columns)
        }
        "temp_file.pg_stat_statements.temp_blocks" => {
            derive_temp_file_statements_insights(rows, columns)
        }
        "query_family.explain"
        | "temp_file.explain"
        | "query_family.explain_analyze"
        | "temp_file.explain_analyze" => derive_explain_insights(rows, columns),
        _ => Vec::new(),
    }
}

fn derive_explain_insights(
    rows: &[Vec<serde_json::Value>],
    _columns: &[String],
) -> Vec<SqlActionInsight> {
    let mut insights = Vec::new();
    let mut plan_text = String::new();

    for row in rows {
        if let Some(first_val) = row.first() {
            if let Some(line) = first_val.as_str() {
                plan_text.push_str(line);
                plan_text.push('\n');
            }
        }
    }

    let has_disk_sort = plan_text.contains("external merge") || plan_text.contains("Disk:");
    if has_disk_sort {
        let detail = if let Some(idx) = plan_text.find("Disk:") {
            let rest = &plan_text[idx..];
            let end_line = rest.find('\n').unwrap_or(rest.len());
            format!("spilled to disk ({})", &rest[..end_line])
        } else {
            "spilled to disk (external merge)".to_string()
        };

        insights.push(SqlActionInsight {
            insight_id: "query_plan_disk_spill_detected".to_string(),
            label: "Query plan disk spill detected".to_string(),
            confidence: SqlInsightConfidence::High,
            reason: format!(
                "The query execution plan confirms a sort or hash operation {}. This explains the temporary files generated by this query.",
                detail
            ),
        });
    }

    if plan_text.contains("temp read=") || plan_text.contains("temp written=") {
        let mut temp_read = 0;
        let mut temp_written = 0;

        if let Some(idx) = plan_text.find("temp read=") {
            let sub = &plan_text[idx + 10..];
            let num_str: String = sub.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(n) = num_str.parse::<i64>() {
                temp_read = n;
            }
        }
        if let Some(idx) = plan_text.find("temp written=") {
            let sub = &plan_text[idx + 13..];
            let num_str: String = sub.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(n) = num_str.parse::<i64>() {
                temp_written = n;
            }
        }

        insights.push(SqlActionInsight {
            insight_id: "explain_analyze_temp_buffers".to_string(),
            label: "Temp buffers written to disk detected".to_string(),
            confidence: SqlInsightConfidence::High,
            reason: format!(
                "EXPLAIN (ANALYZE, BUFFERS) shows temp buffers read: {} blocks, written: {} blocks, confirming temporary files are actively written during execution.",
                temp_read, temp_written
            ),
        });
    }

    if insights.is_empty() && !plan_text.is_empty() {
        insights.push(SqlActionInsight {
            insight_id: "query_plan_no_disk_spill".to_string(),
            label: "Query plan shows no disk spill in current state".to_string(),
            confidence: SqlInsightConfidence::Medium,
            reason: "The execution plan does not show any active disk spill. This might be because the current query parameters or database size do not trigger a disk sort in this test environment.".to_string(),
        });
    }

    insights
}

fn derive_temp_file_database_insights(
    rows: &[Vec<serde_json::Value>],
    columns: &[String],
) -> Vec<SqlActionInsight> {
    let mut insights = Vec::new();
    let temp_files_idx = columns.iter().position(|c| c == "temp_files");
    let temp_bytes_idx = columns.iter().position(|c| c == "temp_bytes");
    let datname_idx = columns.iter().position(|c| c == "datname");

    if let (Some(tf_idx), Some(tb_idx)) = (temp_files_idx, temp_bytes_idx) {
        for row in rows {
            if let (Some(tf_val), Some(tb_val)) = (row.get(tf_idx), row.get(tb_idx)) {
                let temp_files = tf_val.as_i64().unwrap_or(0);
                let temp_bytes = tb_val.as_i64().unwrap_or(0);
                let db_name = datname_idx
                    .and_then(|idx| row.get(idx))
                    .and_then(|v| v.as_str())
                    .unwrap_or("database");

                if temp_files > 0 {
                    insights.push(SqlActionInsight {
                        insight_id: "temp_files_volume_detected".to_string(),
                        label: "Temporary files detected in pg_stat_database".to_string(),
                        confidence: SqlInsightConfidence::High,
                        reason: format!(
                            "Database '{}' has accumulated {} temporary files totaling {} bytes. This confirms disk-write pressure for sorting or hashing operations.",
                            db_name, temp_files, temp_bytes
                        ),
                    });
                }
            }
        }
    }

    insights
}

fn derive_temp_file_statements_insights(
    rows: &[Vec<serde_json::Value>],
    columns: &[String],
) -> Vec<SqlActionInsight> {
    let mut insights = Vec::new();
    let temp_read_idx = columns.iter().position(|c| c == "temp_blks_read");
    let temp_write_idx = columns.iter().position(|c| c == "temp_blks_written");
    let query_idx = columns.iter().position(|c| c == "query");

    if let (Some(tr_idx), Some(tw_idx)) = (temp_read_idx, temp_write_idx) {
        let mut total_queries_writing = 0;
        let mut max_temp_blocks = 0;
        let mut max_query = String::new();

        for row in rows {
            let tr = row.get(tr_idx).and_then(|v| v.as_i64()).unwrap_or(0);
            let tw = row.get(tw_idx).and_then(|v| v.as_i64()).unwrap_or(0);
            let total = tr + tw;
            if total > 0 {
                total_queries_writing += 1;
                if total > max_temp_blocks {
                    max_temp_blocks = total;
                    max_query = query_idx
                        .and_then(|idx| row.get(idx))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                }
            }
        }

        if total_queries_writing > 0 {
            insights.push(SqlActionInsight {
                insight_id: "statements_writing_temp_blocks".to_string(),
                label: "Statements writing temporary blocks detected".to_string(),
                confidence: SqlInsightConfidence::High,
                reason: format!(
                    "Found {} query/queries writing to temporary blocks. The heaviest query wrote/read {} blocks: '{}'.",
                    total_queries_writing, max_temp_blocks, max_query
                ),
            });
        }
    }

    insights
}

fn derive_pg_stat_activity_dimension_insights(
    rows: &[Vec<serde_json::Value>],
    columns: &[String],
) -> Vec<SqlActionInsight> {
    if columns.is_empty() {
        return Vec::new();
    }

    if rows.is_empty() {
        return vec![SqlActionInsight {
            insight_id: "no_live_match_found".to_string(),
            label: "No matching live sessions found".to_string(),
            confidence: SqlInsightConfidence::High,
            reason:
                "The bounded pg_stat_activity lookup returned no non-idle sessions for the parent finding dimensions."
                    .to_string(),
        }];
    }

    let index_by_name: BTreeMap<&str, usize> = columns
        .iter()
        .enumerate()
        .map(|(index, name)| (name.as_str(), index))
        .collect();
    let state_idx = index_by_name.get("state").copied();
    let wait_event_type_idx = index_by_name.get("wait_event_type").copied();
    let wait_event_idx = index_by_name.get("wait_event").copied();

    let mut ctx = ActivityInsightContext {
        row_count: rows.len(),
        ..Default::default()
    };

    for row in rows {
        if value_eq(row, state_idx, "active") {
            ctx.active_rows += 1;
        }
        if value_eq(row, wait_event_type_idx, "Lock") {
            ctx.lock_wait_rows += 1;
        }
        if value_eq(row, wait_event_type_idx, "Lock")
            && value_eq(row, wait_event_idx, "transactionid")
        {
            ctx.transactionid_lock_rows += 1;
        }
        if value_eq(row, wait_event_type_idx, "IO") {
            ctx.io_wait_rows += 1;
        }
        if value_eq(row, wait_event_type_idx, "Client") {
            ctx.client_wait_rows += 1;
        }
    }

    let mut insights = Vec::new();
    insights.push(SqlActionInsight {
        insight_id: "live_match_found".to_string(),
        label: "Matching live sessions found".to_string(),
        confidence: SqlInsightConfidence::High,
        reason: if ctx.row_count == 1 {
            "The bounded pg_stat_activity lookup found one non-idle session matching the parent finding dimensions.".to_string()
        } else {
            format!(
                "The bounded pg_stat_activity lookup found {} non-idle sessions matching the parent finding dimensions.",
                ctx.row_count
            )
        },
    });

    if ctx.row_count > 1 {
        insights.push(SqlActionInsight {
            insight_id: "multiple_matching_sessions".to_string(),
            label: "Multiple matching sessions are active".to_string(),
            confidence: SqlInsightConfidence::High,
            reason: format!(
                "{} matching sessions are currently active for the parent finding dimensions.",
                ctx.row_count
            ),
        });
    }

    if ctx.active_rows > 0 {
        insights.push(SqlActionInsight {
            insight_id: "active_session_present".to_string(),
            label: "The query family appears live now".to_string(),
            confidence: SqlInsightConfidence::High,
            reason: if ctx.active_rows == 1 {
                "At least one matching session is in state=active, so the historical finding also appears to be a current live issue.".to_string()
            } else {
                format!(
                    "{} matching sessions are in state=active, so the historical finding also appears to be a current live issue.",
                    ctx.active_rows
                )
            },
        });
    }

    if ctx.transactionid_lock_rows > 0 {
        insights.push(SqlActionInsight {
            insight_id: "transactionid_lock_wait".to_string(),
            label: "The query appears blocked on another transaction".to_string(),
            confidence: SqlInsightConfidence::High,
            reason: if ctx.transactionid_lock_rows == 1 {
                "A matching active session is waiting on wait_event_type=Lock and wait_event=transactionid, which strongly suggests lock contention on another transaction.".to_string()
            } else {
                format!(
                    "{} matching active sessions are waiting on wait_event_type=Lock and wait_event=transactionid, which strongly suggests lock contention on other transactions.",
                    ctx.transactionid_lock_rows
                )
            },
        });
    } else if ctx.lock_wait_rows > 0 {
        insights.push(SqlActionInsight {
            insight_id: "lock_wait_detected".to_string(),
            label: "The query appears blocked on a lock".to_string(),
            confidence: SqlInsightConfidence::High,
            reason: if ctx.lock_wait_rows == 1 {
                "A matching active session is waiting on wait_event_type=Lock, which suggests lock contention rather than pure execution cost.".to_string()
            } else {
                format!(
                    "{} matching active sessions are waiting on wait_event_type=Lock, which suggests lock contention rather than pure execution cost.",
                    ctx.lock_wait_rows
                )
            },
        });
    }

    if ctx.io_wait_rows > 0 {
        insights.push(SqlActionInsight {
            insight_id: "io_wait_detected".to_string(),
            label: "The query appears to be waiting on IO".to_string(),
            confidence: SqlInsightConfidence::Medium,
            reason: if ctx.io_wait_rows == 1 {
                "A matching active session is waiting on wait_event_type=IO, which suggests an IO or storage bottleneck.".to_string()
            } else {
                format!(
                    "{} matching active sessions are waiting on wait_event_type=IO, which suggests an IO or storage bottleneck.",
                    ctx.io_wait_rows
                )
            },
        });
    }

    if ctx.client_wait_rows > 0 {
        insights.push(SqlActionInsight {
            insight_id: "client_wait_detected".to_string(),
            label: "The session appears to be waiting on the client".to_string(),
            confidence: SqlInsightConfidence::Medium,
            reason: if ctx.client_wait_rows == 1 {
                "A matching active session is waiting on wait_event_type=Client, which suggests the bottleneck may be outside PostgreSQL execution itself.".to_string()
            } else {
                format!(
                    "{} matching active sessions are waiting on wait_event_type=Client, which suggests the bottleneck may be outside PostgreSQL execution itself.",
                    ctx.client_wait_rows
                )
            },
        });
    }

    insights
}

fn value_eq(row: &[serde_json::Value], index: Option<usize>, expected: &str) -> bool {
    let Some(index) = index else {
        return false;
    };
    matches!(row.get(index), Some(serde_json::Value::String(value)) if value == expected)
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
        "query_family.explain" | "temp_file.explain" => {
            let sql = if let Some(qf) = &finding.query_family {
                qf.normalized_sql.clone()
            } else if let Some(tf) = &finding.temp_file {
                tf.normalized_sql.as_deref().unwrap_or("").to_string()
            } else {
                return Err(PgLogstatsError::Configuration {
                    message: format!("Action '{}' requires query text", action.action_id),
                    field: Some("selected_action_id".to_string()),
                });
            };
            if sql.is_empty() {
                return Err(PgLogstatsError::Configuration {
                    message: format!("Action '{}' requires non-empty query text", action.action_id),
                    field: Some("selected_action_id".to_string()),
                });
            }
            Ok(PreparedQuery {
                sql: format!("EXPLAIN {};", sql),
                parameters: vec![],
            })
        }
        "query_family.explain_analyze" | "temp_file.explain_analyze" => {
            let sql = if let Some(qf) = &finding.query_family {
                qf.normalized_sql.clone()
            } else if let Some(tf) = &finding.temp_file {
                tf.normalized_sql.as_deref().unwrap_or("").to_string()
            } else {
                return Err(PgLogstatsError::Configuration {
                    message: format!("Action '{}' requires query text", action.action_id),
                    field: Some("selected_action_id".to_string()),
                });
            };
            if sql.is_empty() {
                return Err(PgLogstatsError::Configuration {
                    message: format!("Action '{}' requires non-empty query text", action.action_id),
                    field: Some("selected_action_id".to_string()),
                });
            }
            Ok(PreparedQuery {
                sql: format!("EXPLAIN (ANALYZE, BUFFERS) {};", sql),
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
    use crate::guidance::GuidancePayload;
    use crate::triage::{NextAction, NextActionPriority, NextActionType, Verdict};

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
                label: "test".to_string(),
                status: NextActionStatus::Allowed,
                priority: NextActionPriority::Recommended,
                judgement_required: true,
                reason: "test".to_string(),
                target_id: Some("demo".to_string()),
                command: None,
                survey: None,
                parameters: None,
                risk: None,
                action_class: None,
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

    #[test]
    fn pg_stat_activity_insights_detect_lock_wait_patterns() {
        let columns = vec![
            "pid".to_string(),
            "usename".to_string(),
            "datname".to_string(),
            "application_name".to_string(),
            "state".to_string(),
            "wait_event_type".to_string(),
            "wait_event".to_string(),
            "query_start".to_string(),
            "query_id".to_string(),
            "query".to_string(),
        ];
        let rows = vec![vec![
            serde_json::json!(42137),
            serde_json::json!("app"),
            serde_json::json!("appdb"),
            serde_json::json!("api"),
            serde_json::json!("active"),
            serde_json::json!("Lock"),
            serde_json::json!("transactionid"),
            serde_json::json!("2026-06-14T10:00:18+00:00"),
            serde_json::Value::Null,
            serde_json::json!(
                "SELECT * FROM invoices WHERE workspace_id = $1 ORDER BY created_at DESC LIMIT $2"
            ),
        ]];

        let insights = derive_sql_action_insights(
            "query_family.pg_stat_activity.by_dimensions:qf_demo",
            &columns,
            &rows,
        );
        let ids: Vec<_> = insights.iter().map(|i| i.insight_id.as_str()).collect();

        assert!(ids.contains(&"live_match_found"));
        assert!(ids.contains(&"active_session_present"));
        assert!(ids.contains(&"transactionid_lock_wait"));
    }

    #[test]
    fn pg_stat_activity_insights_report_no_live_match() {
        let columns = vec!["state".to_string(), "wait_event_type".to_string()];
        let insights = derive_sql_action_insights(
            "query_family.pg_stat_activity.by_dimensions:qf_demo",
            &columns,
            &[],
        );

        assert_eq!(insights.len(), 1);
        assert_eq!(insights[0].insight_id, "no_live_match_found");
        assert_eq!(insights[0].confidence, SqlInsightConfidence::High);
    }

    #[test]
    fn sql_insights_are_not_forced_for_unknown_actions() {
        let columns = vec!["state".to_string(), "wait_event_type".to_string()];
        let rows = vec![vec![serde_json::json!("active"), serde_json::json!("Lock")]];
        let insights = derive_sql_action_insights(
            "query_family.pg_stat_statements.by_queryid:demo",
            &columns,
            &rows,
        );

        assert!(insights.is_empty());
    }

    #[test]
    fn explain_sql_generation() {
        let selected = selected_query_family_action("query_family.explain:demo");
        let prepared =
            prepare_findings_workflow_sql(&selected.action, &selected.finding, &[]).unwrap();
        assert_eq!(prepared.sql, "EXPLAIN SELECT 1;");
        assert!(prepared.parameters.is_empty());
    }

    #[test]
    fn explain_analyze_sql_generation() {
        let selected = selected_query_family_action("query_family.explain_analyze:demo");
        let prepared =
            prepare_findings_workflow_sql(&selected.action, &selected.finding, &[]).unwrap();
        assert_eq!(prepared.sql, "EXPLAIN (ANALYZE, BUFFERS) SELECT 1;");
        assert!(prepared.parameters.is_empty());
    }

    #[test]
    fn temp_file_insights_and_remedies() {
        let columns = vec![
            "datname".to_string(),
            "temp_files".to_string(),
            "temp_bytes".to_string(),
        ];
        let rows = vec![vec![
            serde_json::json!("appdb"),
            serde_json::json!(42),
            serde_json::json!(35651584000i64),
        ]];

        let insights = derive_sql_action_insights(
            "temp_file.pg_stat_database.temp_counters:qf_demo",
            &columns,
            &rows,
        );

        assert_eq!(insights.len(), 1);
        assert_eq!(insights[0].insight_id, "temp_files_volume_detected");
        assert_eq!(insights[0].confidence, SqlInsightConfidence::High);
        assert!(insights[0].reason.contains("35651584000 bytes"));

        let payload = SqlActionPayload {
            action_id: "temp_file.pg_stat_database.temp_counters:qf_demo".to_string(),
            source_report_id: None,
            source_finding_id: Some("qf_demo".to_string()),
            insights,
            row_count: 1,
            truncated: false,
            columns,
            rows,
        };

        let next_actions = payload.evaluate_rules(
            OperatingMode::LogBackedAndLive,
            Some(Verdict::Clear),
            &AppConfig::default(),
        );

        let explain_action = next_actions
            .iter()
            .find(|a| a.action_id == "temp_file.pg_stat_statements.temp_blocks")
            .unwrap();
        assert_eq!(explain_action.status, NextActionStatus::Allowed);

        let remedial_action = next_actions
            .iter()
            .find(|a| a.action_id == "remedial.optimize_work_mem")
            .unwrap();
        assert_eq!(remedial_action.status, NextActionStatus::Allowed);
        assert!(remedial_action.reason.contains("SET LOCAL work_mem"));

        let index_action = next_actions
            .iter()
            .find(|a| a.action_id == "remedial.create_sort_index")
            .unwrap();
        assert_eq!(index_action.status, NextActionStatus::Allowed);

        let projection_action = next_actions
            .iter()
            .find(|a| a.action_id == "remedial.reduce_projection_width")
            .unwrap();
        assert_eq!(projection_action.status, NextActionStatus::Allowed);
    }

    #[test]
    fn explain_insights_derive_disk_spills() {
        let columns = vec!["QUERY PLAN".to_string()];
        let rows = vec![
            vec![serde_json::json!("Sort  (cost=123.45..124.45 rows=100 width=40) (actual time=0.12..0.15 rows=100 loops=1)")],
            vec![serde_json::json!("  Sort Key: price DESC")],
            vec![serde_json::json!("  Sort Method: external merge  Disk: 85000kB")],
        ];

        let insights = derive_sql_action_insights("temp_file.explain:qf_demo", &columns, &rows);

        assert_eq!(insights.len(), 1);
        assert_eq!(insights[0].insight_id, "query_plan_disk_spill_detected");
        assert!(insights[0]
            .reason
            .contains("spilled to disk (Disk: 85000kB)"));

        let payload = SqlActionPayload {
            action_id: "temp_file.explain:qf_demo".to_string(),
            source_report_id: None,
            source_finding_id: Some("qf_demo".to_string()),
            insights,
            row_count: 3,
            truncated: false,
            columns,
            rows,
        };

        let next_actions = payload.evaluate_rules(
            OperatingMode::LogBackedAndLive,
            Some(Verdict::Clear),
            &AppConfig::default(),
        );

        let remedial_action = next_actions
            .iter()
            .find(|a| a.action_id == "remedial.optimize_work_mem")
            .unwrap();
        assert_eq!(remedial_action.status, NextActionStatus::Allowed);

        let index_action = next_actions
            .iter()
            .find(|a| a.action_id == "remedial.create_sort_index")
            .unwrap();
        assert_eq!(index_action.status, NextActionStatus::Allowed);

        let projection_action = next_actions
            .iter()
            .find(|a| a.action_id == "remedial.reduce_projection_width")
            .unwrap();
        assert_eq!(projection_action.status, NextActionStatus::Allowed);
    }

    #[test]
    fn explain_analyze_insights_derive_temp_buffers() {
        let columns = vec!["QUERY PLAN".to_string()];
        let rows = vec![
            vec![serde_json::json!("Hash Join  (cost=12.00..34.00 rows=5 width=20) (actual time=1.2..3.4 rows=5 loops=1)")],
            vec![serde_json::json!("  Buffers: shared hit=45 read=12, temp read=125 temp written=250")],
        ];

        let insights =
            derive_sql_action_insights("temp_file.explain_analyze:qf_demo", &columns, &rows);

        assert_eq!(insights.len(), 1);
        assert_eq!(insights[0].insight_id, "explain_analyze_temp_buffers");
        assert!(insights[0]
            .reason
            .contains("temp buffers read: 125 blocks, written: 250 blocks"));
    }
}
