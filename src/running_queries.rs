use crate::config::AppConfig;
use crate::database::{connect_postgres_client, resolve_database_dsn};
use crate::guidance::GuidancePayload;
use crate::inspect::{InspectCheckId, InspectReportPayload};
use crate::triage::{
    ActionClass, ActionKind, CheckStatus, NextAction, NextActionCommand, OperatingMode,
    PgTriageReport, Verdict, PG_TRIAGE_SCHEMA_VERSION,
};
use crate::PgLogstatsError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Detailed aggregate statement statistics from pg_stat_statements.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StatementHistory {
    pub calls: i64,
    pub total_exec_time: f64,
    pub mean_exec_time: f64,
    pub rows: i64,
    pub shared_blks_hit: i64,
    pub shared_blks_read: i64,
    pub temp_blks_read: i64,
    pub temp_blks_written: i64,
}

/// A snapshot of an active/waiting PostgreSQL session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActiveSession {
    pub pid: i32,
    pub database: Option<String>,
    pub user: Option<String>,
    pub application_name: Option<String>,
    pub state: Option<String>,
    pub wait_event_type: Option<String>,
    pub wait_event: Option<String>,
    pub query_start: Option<String>,
    pub duration_ms: Option<i64>,
    pub query_id: Option<i64>,
    pub query_text: Option<String>,
    pub statement_history: Option<StatementHistory>,
}

/// Summary metrics of currently active queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveQuerySummary {
    pub active_session_count: usize,
    pub waiting_session_count: usize,
    pub idle_in_transaction_count: usize,
    pub long_running_query_count: usize,
}

/// A signal indicating potential blocking (e.g., wait events).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockingSignal {
    pub kind: String,
    pub wait_event_type: String,
    pub count: usize,
}

/// Triage payload holding a live-state snapshot of running queries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunningQueriesPayload {
    pub sources: BTreeMap<String, String>,
    pub active_query_summary: ActiveQuerySummary,
    pub blocking_signals: Vec<BlockingSignal>,
    pub active_sessions: Vec<ActiveSession>,
}

impl GuidancePayload for RunningQueriesPayload {
    fn evaluate_rules(
        &self,
        operating_mode: OperatingMode,
        verdict: Option<Verdict>,
        config: &AppConfig,
    ) -> Vec<NextAction> {
        let mut actions = Vec::new();

        let limit_by_pid = config
            .guidance
            .rules
            .get(&crate::guidance::RuleId::RunningQueryPgStatActivityByPid)
            .and_then(|rc| rc.limit)
            .unwrap_or(crate::guidance::DEFAULT_RULE_LIMIT);

        let limit_blocking = config
            .guidance
            .rules
            .get(&crate::guidance::RuleId::RunningQueryBlockingByPid)
            .and_then(|rc| rc.limit)
            .unwrap_or(crate::guidance::DEFAULT_RULE_LIMIT);

        for rule in crate::guidance::running_query_rules() {
            let rule_limit = match rule.rule_id {
                crate::guidance::RuleId::RunningQueryPgStatActivityByPid => limit_by_pid,
                crate::guidance::RuleId::RunningQueryBlockingByPid => limit_blocking,
                _ => crate::guidance::DEFAULT_RULE_LIMIT,
            };

            let mut count = 0;
            for session in &self.active_sessions {
                if count >= rule_limit {
                    break;
                }

                let applies = match rule.rule_id {
                    crate::guidance::RuleId::RunningQueryPgStatActivityByPid => true,
                    crate::guidance::RuleId::RunningQueryBlockingByPid => {
                        session.wait_event_type.is_some()
                    }
                    _ => false,
                };

                if !applies {
                    continue;
                }

                let resolved_rule = rule.clone();
                let mut sql_preview = None;
                let mut command = None;

                if let Some(ref template) = rule.sql_template {
                    let sql = template.replace("$1", &session.pid.to_string());
                    sql_preview = Some(sql);
                    command = Some(NextActionCommand {
                        argv: vec!["pg-logstats".to_string(), "run-sql".to_string()],
                    });
                }

                let (status, reason) = crate::guidance::evaluate_rule_constraints(
                    &resolved_rule,
                    operating_mode,
                    verdict,
                    config,
                );

                let target = Some(session.pid.to_string());
                let action = crate::guidance::build_next_action(
                    &resolved_rule,
                    status,
                    reason,
                    target,
                    command,
                    sql_preview,
                );

                actions.push(action);
                count += 1;
            }
        }

        actions
    }
}

/// Executes the `running-queries` command, querying pg_stat_activity and
/// optionally pg_stat_statements to produce a live database triage report.
pub fn run_running_queries(
    cli_dsn: Option<&str>,
    config: &AppConfig,
    inspect_report: Option<&PgTriageReport<InspectReportPayload>>,
) -> Result<PgTriageReport<RunningQueriesPayload>, PgLogstatsError> {
    let resolved_dsn = resolve_database_dsn(cli_dsn, config).ok_or_else(|| {
        PgLogstatsError::Configuration {
            message: "Database connection not configured. Specify --dsn, PG_LOGSTATS_DATABASE_URL, or [database].dsn in config.".to_string(),
            field: Some("database_connection_not_configured".to_string()),
        }
    })?;

    let mut client = connect_postgres_client(&resolved_dsn, config.database.connect_timeout_ms)
        .map_err(|e| PgLogstatsError::Configuration {
            message: e,
            field: Some("dsn".to_string()),
        })?;

    let now = Utc::now();

    // Query pg_stat_activity, filtering out our own session in SQL
    let rows = client
        .query(
            "SELECT pid, datname, usename, application_name, state, wait_event_type, wait_event, query_start, query_id, query FROM pg_stat_activity WHERE pid != pg_backend_pid();",
            &[],
        )
        .map_err(|e| PgLogstatsError::Unexpected {
            message: format!("Failed to query pg_stat_activity: {}", e),
            context: None,
        })?;

    let mut all_sessions = Vec::new();
    let long_running_threshold = config.running_queries.thresholds.long_running_query_ms as i64;

    for row in rows {
        let pid: i32 = row.get("pid");
        let database: Option<String> = row.get("datname");
        let user: Option<String> = row.get("usename");
        let application_name: Option<String> = row.get("application_name");
        let state: Option<String> = row.get("state");
        let wait_event_type: Option<String> = row.get("wait_event_type");
        let wait_event: Option<String> = row.get("wait_event");
        let query_start: Option<DateTime<Utc>> = row.get("query_start");
        let query_id: Option<i64> = row.get("query_id");
        let mut query_text: Option<String> = row.get("query");

        // Truncate query text if configured
        if let Some(ref mut text) = query_text {
            if let Some(limit) = config.running_queries.query_truncate_len {
                if text.chars().count() > limit {
                    *text = text.chars().take(limit).collect::<String>() + "...";
                }
            }
        }

        // Calculate duration_ms for state = 'active'
        let duration_ms = if state.as_deref() == Some("active") {
            query_start.map(|start| {
                let diff = now.signed_duration_since(start).num_milliseconds();
                diff.max(0)
            })
        } else {
            None
        };

        let query_start_str = query_start.map(|dt| dt.to_rfc3339());

        all_sessions.push(ActiveSession {
            pid,
            database,
            user,
            application_name,
            state,
            wait_event_type,
            wait_event,
            query_start: query_start_str,
            duration_ms,
            query_id,
            query_text,
            statement_history: None,
        });
    }

    // Keep non-active sessions only when they contribute to triage (active, idle in transaction, or waiting)
    let mut retained_sessions: Vec<ActiveSession> = all_sessions
        .iter()
        .filter(|s| {
            s.state.as_deref() == Some("active")
                || s.state.as_deref() == Some("idle in transaction")
                || s.wait_event_type.is_some()
        })
        .cloned()
        .collect();

    // Query pg_stat_statements if available and we have query_ids
    let has_statements = inspect_report.is_some_and(|ir| {
        ir.payload
            .database_inspect
            .checks
            .get(&InspectCheckId::PgStatStatementsProbe)
            .is_some_and(|check| check.status == CheckStatus::Passed)
    });

    let mut sources = BTreeMap::new();
    sources.insert("pg_stat_activity".to_string(), "available".to_string());

    if has_statements {
        sources.insert("pg_stat_statements".to_string(), "available".to_string());
        let query_ids: Vec<i64> = retained_sessions
            .iter()
            .filter_map(|s| s.query_id)
            .collect();

        if !query_ids.is_empty() {
            let stmt_rows = client
                .query(
                    "SELECT queryid, calls, total_exec_time, mean_exec_time, rows, shared_blks_hit, shared_blks_read, temp_blks_read, temp_blks_written FROM pg_stat_statements WHERE queryid = ANY($1);",
                    &[&query_ids],
                )
                .map_err(|e| PgLogstatsError::Unexpected {
                    message: format!("Failed to query pg_stat_statements: {}", e),
                    context: None,
                })?;

            let mut statements_map = BTreeMap::new();
            for r in stmt_rows {
                let qid: i64 = r.get("queryid");
                let calls: i64 = r.get("calls");
                let total_exec_time: f64 = r.get("total_exec_time");
                let mean_exec_time: f64 = r.get("mean_exec_time");
                let rows_cnt: i64 = r.get("rows");
                let shared_blks_hit: i64 = r.get("shared_blks_hit");
                let shared_blks_read: i64 = r.get("shared_blks_read");
                let temp_blks_read: i64 = r.get("temp_blks_read");
                let temp_blks_written: i64 = r.get("temp_blks_written");

                statements_map.insert(
                    qid,
                    StatementHistory {
                        calls,
                        total_exec_time,
                        mean_exec_time,
                        rows: rows_cnt,
                        shared_blks_hit,
                        shared_blks_read,
                        temp_blks_read,
                        temp_blks_written,
                    },
                );
            }

            for session in &mut retained_sessions {
                if let Some(qid) = session.query_id {
                    if let Some(history) = statements_map.get(&qid) {
                        session.statement_history = Some(history.clone());
                    }
                }
            }
        }
    } else {
        sources.insert("pg_stat_statements".to_string(), "unavailable".to_string());
    }

    // Calculate active query summary metrics
    let active_session_count = all_sessions
        .iter()
        .filter(|s| s.state.as_deref() == Some("active"))
        .count();

    let waiting_session_count = all_sessions
        .iter()
        .filter(|s| s.wait_event_type.is_some())
        .count();

    let idle_in_transaction_count = all_sessions
        .iter()
        .filter(|s| s.state.as_deref() == Some("idle in transaction"))
        .count();

    let long_running_query_count = all_sessions
        .iter()
        .filter(|s| {
            s.state.as_deref() == Some("active")
                && s.duration_ms.unwrap_or(0) > long_running_threshold
        })
        .count();

    let active_query_summary = ActiveQuerySummary {
        active_session_count,
        waiting_session_count,
        idle_in_transaction_count,
        long_running_query_count,
    };

    // Calculate blocking signals
    let mut wait_events_counts = BTreeMap::new();
    for session in &all_sessions {
        if let Some(ref wet) = session.wait_event_type {
            *wait_events_counts.entry(wet.clone()).or_insert(0) += 1;
        }
    }

    let blocking_signals: Vec<BlockingSignal> = wait_events_counts
        .into_iter()
        .map(|(wet, count)| BlockingSignal {
            kind: "wait_event".to_string(),
            wait_event_type: wet,
            count,
        })
        .collect();

    // Calculate verdict
    let waiting_lock_or_io_count = all_sessions
        .iter()
        .filter(|s| {
            s.wait_event_type
                .as_ref()
                .is_some_and(|t| t == "Lock" || t == "IO")
        })
        .count();

    let saturated = long_running_query_count >= 1
        && (idle_in_transaction_count >= 1 || waiting_session_count >= 1);

    let busy = long_running_query_count >= 1
        || waiting_lock_or_io_count
            >= config
                .running_queries
                .thresholds
                .waiting_session_count_threshold as usize
        || idle_in_transaction_count
            >= config
                .running_queries
                .thresholds
                .idle_in_transaction_count_threshold as usize;

    let verdict = if saturated {
        Verdict::Saturated
    } else if busy {
        Verdict::Busy
    } else {
        Verdict::Clear
    };

    let mut verdict_reasons = Vec::new();
    if long_running_query_count >= 1 {
        verdict_reasons.push("long_running_queries_present".to_string());
    }
    if waiting_session_count >= 1 {
        verdict_reasons.push("waiting_sessions_present".to_string());
    }
    if idle_in_transaction_count >= 1 {
        verdict_reasons.push("idle_in_transaction_sessions_present".to_string());
    }

    // Allowed and blocked action classes based on verdict
    let (allowed_actions, blocked_actions) = match verdict {
        Verdict::Clear => (
            Some(vec![
                ActionClass::SystemCatalogReads,
                ActionClass::StatsViewReads,
                ActionClass::BoundedActivityQueries,
                ActionClass::TextPatternStatsSearch,
                ActionClass::ExplainWithoutAnalyze,
            ]),
            Some(vec![
                ActionClass::LargeUnboundedSelects,
                ActionClass::ExplainAnalyze,
                ActionClass::WriteOrAdminAction,
            ]),
        ),
        Verdict::Busy => (
            Some(vec![
                ActionClass::SystemCatalogReads,
                ActionClass::StatsViewReads,
                ActionClass::BoundedActivityQueries,
            ]),
            Some(vec![
                ActionClass::TextPatternStatsSearch,
                ActionClass::ExplainWithoutAnalyze,
                ActionClass::LargeUnboundedSelects,
                ActionClass::ExplainAnalyze,
                ActionClass::WriteOrAdminAction,
            ]),
        ),
        Verdict::Saturated => (
            Some(vec![]),
            Some(vec![
                ActionClass::SystemCatalogReads,
                ActionClass::StatsViewReads,
                ActionClass::BoundedActivityQueries,
                ActionClass::TextPatternStatsSearch,
                ActionClass::ExplainWithoutAnalyze,
                ActionClass::LargeUnboundedSelects,
                ActionClass::ExplainAnalyze,
                ActionClass::WriteOrAdminAction,
            ]),
        ),
        Verdict::Unknown => (None, None),
    };

    let operating_mode = inspect_report
        .map(|ir| ir.operating_mode)
        .unwrap_or(OperatingMode::LiveOnly);

    let payload = RunningQueriesPayload {
        sources,
        active_query_summary,
        blocking_signals,
        active_sessions: retained_sessions,
    };

    let mut report = PgTriageReport {
        schema_version: PG_TRIAGE_SCHEMA_VERSION,
        workflow: ActionKind::RunningQueries,
        operating_mode,
        limitations: Vec::new(),
        verdict: Some(verdict),
        verdict_reasons,
        allowed_actions,
        blocked_actions,
        analysis_window: None,
        source_summary: None,
        next_actions: Vec::new(),
        report_id: None,
        parent_report_id: None,
        selected_action_id: None,
        created_at: None,
        payload,
    };

    crate::guidance::populate_next_actions(&mut report, config);

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::triage::{NextActionStatus, Verdict};
    use crate::AppConfig;

    fn sample_running_queries_payload() -> RunningQueriesPayload {
        let mut sources = BTreeMap::new();
        sources.insert("pg_stat_activity".to_string(), "available".to_string());

        let active_query_summary = ActiveQuerySummary {
            active_session_count: 2,
            waiting_session_count: 1,
            idle_in_transaction_count: 0,
            long_running_query_count: 0,
        };

        let blocking_signals = vec![BlockingSignal {
            kind: "wait_event".to_string(),
            wait_event_type: "Lock".to_string(),
            count: 1,
        }];

        let active_sessions = vec![
            ActiveSession {
                pid: 1001,
                database: Some("testdb".to_string()),
                user: Some("testuser".to_string()),
                application_name: Some("testapp".to_string()),
                state: Some("active".to_string()),
                wait_event_type: None,
                wait_event: None,
                query_start: Some("2026-06-12T13:00:00Z".to_string()),
                duration_ms: Some(100),
                query_id: Some(42),
                query_text: Some("SELECT * FROM users;".to_string()),
                statement_history: None,
            },
            ActiveSession {
                pid: 1002,
                database: Some("testdb".to_string()),
                user: Some("testuser".to_string()),
                application_name: Some("testapp".to_string()),
                state: Some("active".to_string()),
                wait_event_type: Some("Lock".to_string()),
                wait_event: Some("relation".to_string()),
                query_start: Some("2026-06-12T13:01:00Z".to_string()),
                duration_ms: Some(200),
                query_id: Some(43),
                query_text: Some("UPDATE users SET name = 'foo' WHERE id = 1;".to_string()),
                statement_history: None,
            },
        ];

        RunningQueriesPayload {
            sources,
            active_query_summary,
            blocking_signals,
            active_sessions,
        }
    }

    #[test]
    fn test_running_queries_rules_emit_pid_and_query_id_actions() {
        let payload = sample_running_queries_payload();
        let config = AppConfig::default();
        let actions =
            payload.evaluate_rules(OperatingMode::LiveOnly, Some(Verdict::Clear), &config);

        // Expect two actions for PID 1001 (by_pid), and two for PID 1002 (by_pid and blocking.by_pid)
        let by_pid_actions: Vec<_> = actions
            .iter()
            .filter(|a| {
                a.action_id
                    .starts_with("running_query.pg_stat_activity.by_pid")
            })
            .collect();
        assert_eq!(by_pid_actions.len(), 2);
        assert!(by_pid_actions
            .iter()
            .any(|a| a.target.as_deref() == Some("1001")));
        assert!(by_pid_actions
            .iter()
            .any(|a| a.target.as_deref() == Some("1002")));

        let blocking_actions: Vec<_> = actions
            .iter()
            .filter(|a| a.action_id.starts_with("running_query.blocking.by_pid"))
            .collect();
        assert_eq!(blocking_actions.len(), 1);
        assert_eq!(blocking_actions[0].target.as_deref(), Some("1002"));
    }

    #[test]
    fn test_running_queries_rules_respect_verdict_policy() {
        let payload = sample_running_queries_payload();
        let config = AppConfig::default();

        // 1. With Verdict::Clear, these actions should be allowed (as BoundedActivityQueries is allowed)
        let actions_clear =
            payload.evaluate_rules(OperatingMode::LiveOnly, Some(Verdict::Clear), &config);
        for action in &actions_clear {
            assert_eq!(action.status, NextActionStatus::Allowed);
        }

        // 2. With Verdict::Saturated, these actions should be blocked by verdict
        let actions_saturated =
            payload.evaluate_rules(OperatingMode::LiveOnly, Some(Verdict::Saturated), &config);
        for action in &actions_saturated {
            assert_eq!(action.status, NextActionStatus::BlockedByVerdict);
        }
    }
}
