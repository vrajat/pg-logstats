use crate::config::{AgentInstallTargetConfig, AppConfig};
use crate::triage::{
    CheckStatus, OperatingMode, PgTriageReport, WorkflowId, PG_TRIAGE_SCHEMA_VERSION,
};
use crate::{normalize_log_entries, Correlator, EventSourceKind, LogEntry, ProcessOrderCorrelator};
use postgres::{Client, NoTls};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration;

const DATABASE_URL_ENV_VAR: &str = "PG_LOGSTATS_DATABASE_URL";

const REQUIRED_CHECKS: &[&str] = &[
    "log_source_reachable",
    "statement_evidence",
    "duration_evidence",
    "correlation_evidence",
    "log_destination",
    "log_line_prefix",
    "log_duration",
    "log_min_duration_statement",
    "log_temp_files",
    "track_activities",
    "shared_preload_libraries",
    "compute_query_id",
    "pg_stat_statements_extension",
    "pg_read_all_stats",
    "pg_stat_activity_probe",
    "pg_stat_statements_probe",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessReportPayload {
    pub readiness: ReadinessDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessDetails {
    pub database_readiness: DatabaseReadiness,
    pub agent_readiness: AgentReadiness,
    pub required_checks: Vec<String>,
    pub failed_checks: Vec<String>,
    pub recommended_next_commands: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseReadiness {
    pub mode_candidate: OperatingMode,
    pub checks: BTreeMap<String, ReadinessCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentReadiness {
    pub codex: AgentTargetReadiness,
    pub claude: AgentTargetReadiness,
    pub gemini: AgentTargetReadiness,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTargetReadiness {
    pub status: CheckStatus,
    pub installed: bool,
    pub install_location: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessCheck {
    pub status: CheckStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LogEvidence {
    pub entries: Vec<LogEntry>,
    pub source_kind: EventSourceKind,
}

#[derive(Debug, Clone)]
pub enum LogReadinessEvidence {
    NotRequested,
    Unreachable { reason: String },
    Available(LogEvidence),
}

pub fn database_url_env_var_name() -> &'static str {
    DATABASE_URL_ENV_VAR
}

pub fn resolve_database_dsn(explicit_dsn: Option<&str>, config: &AppConfig) -> Option<String> {
    explicit_dsn
        .map(str::to_string)
        .or_else(|| env::var(DATABASE_URL_ENV_VAR).ok())
        .or_else(|| config.database.dsn.clone())
}

pub fn build_readiness_report(
    config: &AppConfig,
    explicit_dsn: Option<&str>,
    log_evidence: LogReadinessEvidence,
) -> PgTriageReport<ReadinessReportPayload> {
    let mut checks = build_log_checks(log_evidence);
    merge_checks(
        &mut checks,
        build_database_checks(resolve_database_dsn(explicit_dsn, config), config),
    );

    let mode_candidate = determine_mode(&checks);
    let failed_checks = collect_failed_checks(&checks);
    let limitations = build_limitations(mode_candidate, &checks);
    let recommended_next_commands = recommended_next_commands(mode_candidate);
    let agent_readiness = detect_agent_readiness(config);

    PgTriageReport {
        schema_version: PG_TRIAGE_SCHEMA_VERSION,
        workflow: WorkflowId::Readiness,
        operating_mode: mode_candidate,
        limitations,
        verdict: None,
        verdict_reasons: Vec::new(),
        allowed_actions: None,
        blocked_actions: None,
        analysis_window: None,
        source_summary: None,
        payload: ReadinessReportPayload {
            readiness: ReadinessDetails {
                database_readiness: DatabaseReadiness {
                    mode_candidate,
                    checks,
                },
                agent_readiness,
                required_checks: REQUIRED_CHECKS
                    .iter()
                    .map(|name| (*name).to_string())
                    .collect(),
                failed_checks,
                recommended_next_commands,
            },
        },
    }
}

pub fn format_readiness_text(report: &PgTriageReport<ReadinessReportPayload>) -> String {
    let readiness = &report.payload.readiness;
    let mut output = String::new();
    output.push_str(&format!("Operating Mode: {:?}\n", report.operating_mode));

    if report.limitations.is_empty() {
        output.push_str("Limitations: none\n");
    } else {
        output.push_str("Limitations:\n");
        for limitation in &report.limitations {
            output.push_str(&format!("- {limitation}\n"));
        }
    }

    output.push_str("Checks:\n");
    for (name, check) in &readiness.database_readiness.checks {
        output.push_str(&format!("- {name}: {}\n", check_status_label(check.status)));
        if let Some(value) = &check.value {
            output.push_str(&format!("  value: {value}\n"));
        }
        if let Some(reason) = &check.reason {
            output.push_str(&format!("  reason: {reason}\n"));
        }
    }

    output.push_str("Agent Readiness:\n");
    for (name, target) in [
        ("codex", &readiness.agent_readiness.codex),
        ("claude", &readiness.agent_readiness.claude),
        ("gemini", &readiness.agent_readiness.gemini),
    ] {
        output.push_str(&format!(
            "- {name}: {} ({})\n",
            check_status_label(target.status),
            target.install_location
        ));
    }

    if !readiness.recommended_next_commands.is_empty() {
        output.push_str("Recommended Next Commands:\n");
        for command in &readiness.recommended_next_commands {
            output.push_str(&format!("- {command}\n"));
        }
    }

    output
}

fn build_log_checks(log_evidence: LogReadinessEvidence) -> BTreeMap<String, ReadinessCheck> {
    let mut checks = BTreeMap::new();
    let log_evidence = match log_evidence {
        LogReadinessEvidence::NotRequested => {
            for name in [
                "log_source_reachable",
                "statement_evidence",
                "duration_evidence",
                "correlation_evidence",
            ] {
                checks.insert(
                    name.to_string(),
                    skipped_check(Some("log_source_not_requested"), None),
                );
            }
            return checks;
        }
        LogReadinessEvidence::Unreachable { reason } => {
            checks.insert(
                "log_source_reachable".to_string(),
                failed_check(None, Some(reason.as_str())),
            );
            for name in [
                "statement_evidence",
                "duration_evidence",
                "correlation_evidence",
            ] {
                checks.insert(
                    name.to_string(),
                    skipped_check(Some("log_source_unreachable"), None),
                );
            }
            return checks;
        }
        LogReadinessEvidence::Available(log_evidence) => log_evidence,
    };

    let entries = &log_evidence.entries;
    let events = normalize_log_entries(entries, log_evidence.source_kind);
    let executions = ProcessOrderCorrelator.correlate(&events);
    let has_statement = entries.iter().any(LogEntry::is_query);
    let has_duration = entries.iter().any(LogEntry::is_duration);
    let has_correlated_execution = executions
        .iter()
        .any(|execution| execution.duration_ms.is_some());

    checks.insert(
        "log_source_reachable".to_string(),
        passed_check(Some(json!(entries.len())), None),
    );
    checks.insert(
        "statement_evidence".to_string(),
        if has_statement {
            passed_check(Some(json!(true)), None)
        } else {
            failed_check(Some(json!(false)), Some("statement_evidence_missing"))
        },
    );
    checks.insert(
        "duration_evidence".to_string(),
        if has_duration {
            passed_check(Some(json!(true)), None)
        } else {
            failed_check(Some(json!(false)), Some("duration_evidence_missing"))
        },
    );
    checks.insert(
        "correlation_evidence".to_string(),
        if has_correlated_execution {
            passed_check(Some(json!(true)), None)
        } else {
            failed_check(Some(json!(false)), Some("correlation_evidence_missing"))
        },
    );

    checks
}

fn build_database_checks(
    dsn: Option<String>,
    config: &AppConfig,
) -> BTreeMap<String, ReadinessCheck> {
    let mut checks = BTreeMap::new();
    let Some(dsn) = dsn else {
        for name in [
            "log_destination",
            "log_line_prefix",
            "log_duration",
            "log_min_duration_statement",
            "log_temp_files",
            "track_activities",
            "shared_preload_libraries",
            "compute_query_id",
            "pg_stat_statements_extension",
            "pg_read_all_stats",
            "pg_stat_activity_probe",
            "pg_stat_statements_probe",
        ] {
            checks.insert(
                name.to_string(),
                skipped_check(Some("database_connection_not_configured"), None),
            );
        }
        return checks;
    };

    let mut client = match connect_client(&dsn, config.database.connect_timeout_ms) {
        Ok(client) => client,
        Err(reason) => {
            for name in [
                "log_destination",
                "log_line_prefix",
                "log_duration",
                "log_min_duration_statement",
                "log_temp_files",
                "track_activities",
                "shared_preload_libraries",
                "compute_query_id",
                "pg_stat_statements_extension",
                "pg_read_all_stats",
                "pg_stat_activity_probe",
                "pg_stat_statements_probe",
            ] {
                checks.insert(name.to_string(), failed_check(None, Some(reason.as_str())));
            }
            return checks;
        }
    };

    let log_destination = show_setting(&mut client, "log_destination");
    checks.insert(
        "log_destination".to_string(),
        evaluate_log_destination(log_destination.as_deref()),
    );

    let log_line_prefix = show_setting(&mut client, "log_line_prefix");
    checks.insert(
        "log_line_prefix".to_string(),
        evaluate_log_line_prefix(log_line_prefix.as_deref()),
    );

    let log_duration = show_setting(&mut client, "log_duration");
    checks.insert(
        "log_duration".to_string(),
        evaluate_on_off_setting(log_duration.as_deref(), "log_duration_disabled"),
    );

    let log_min_duration_statement = show_setting(&mut client, "log_min_duration_statement");
    checks.insert(
        "log_min_duration_statement".to_string(),
        evaluate_non_negative_setting(
            log_min_duration_statement.as_deref(),
            "log_min_duration_statement_disabled",
        ),
    );

    let log_temp_files = show_setting(&mut client, "log_temp_files");
    checks.insert(
        "log_temp_files".to_string(),
        evaluate_non_negative_setting(log_temp_files.as_deref(), "log_temp_files_disabled"),
    );

    let track_activities = show_setting(&mut client, "track_activities");
    checks.insert(
        "track_activities".to_string(),
        evaluate_on_off_setting(track_activities.as_deref(), "track_activities_disabled"),
    );

    let shared_preload_libraries = show_setting(&mut client, "shared_preload_libraries");
    checks.insert(
        "shared_preload_libraries".to_string(),
        evaluate_list_contains(
            shared_preload_libraries.as_deref(),
            "pg_stat_statements",
            "pg_stat_statements_not_preloaded",
        ),
    );

    let compute_query_id = show_setting(&mut client, "compute_query_id");
    checks.insert(
        "compute_query_id".to_string(),
        evaluate_allowed_values(
            compute_query_id.as_deref(),
            &["auto", "on"],
            "compute_query_id_disabled",
        ),
    );

    checks.insert(
        "pg_stat_statements_extension".to_string(),
        evaluate_exists_query(
            &mut client,
            "SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'pg_stat_statements')",
            "pg_stat_statements_extension_missing",
        ),
    );

    checks.insert(
        "pg_read_all_stats".to_string(),
        evaluate_exists_query(
            &mut client,
            "SELECT pg_has_role(current_user, 'pg_read_all_stats', 'member') OR (SELECT rolsuper FROM pg_roles WHERE rolname = current_user)",
            "pg_read_all_stats_unavailable",
        ),
    );

    checks.insert(
        "pg_stat_activity_probe".to_string(),
        evaluate_probe_query(
            &mut client,
            "SELECT pid, datname, usename, application_name, state, wait_event_type, wait_event, query_id, query FROM pg_stat_activity LIMIT 1",
        ),
    );

    checks.insert(
        "pg_stat_statements_probe".to_string(),
        evaluate_probe_query(
            &mut client,
            "SELECT queryid, query, calls, total_exec_time, mean_exec_time FROM pg_stat_statements ORDER BY total_exec_time DESC LIMIT 1",
        ),
    );

    checks
}

fn detect_agent_readiness(config: &AppConfig) -> AgentReadiness {
    AgentReadiness {
        codex: detect_target(
            config.agent_install.codex.agents_md_path.clone(),
            default_home_path("AGENTS.md"),
        ),
        claude: detect_target(
            target_artifact_path(&config.agent_install.claude, |dir| {
                dir.join("pg-logstats-triage").join("SKILL.md")
            }),
            default_home_path(".claude/skills/pg-logstats-triage/SKILL.md"),
        ),
        gemini: detect_target(
            target_artifact_path(&config.agent_install.gemini, |dir| {
                dir.join("pg-logstats-triage.toml")
            }),
            default_home_path(".gemini/commands/pg-logstats-triage.toml"),
        ),
    }
}

fn target_artifact_path(
    target: &AgentInstallTargetConfig,
    with_dir: impl FnOnce(&Path) -> PathBuf,
) -> Option<PathBuf> {
    if let Some(commands_dir) = &target.commands_dir {
        return Some(with_dir(commands_dir));
    }
    if let Some(skill_dir) = &target.skill_dir {
        return Some(with_dir(skill_dir));
    }
    None
}

fn detect_target(configured: Option<PathBuf>, default: Option<PathBuf>) -> AgentTargetReadiness {
    let path = configured.or(default);
    let installed = path.as_ref().is_some_and(|path| path.exists());
    let install_location = path
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "unconfigured".to_string());

    AgentTargetReadiness {
        status: if installed {
            CheckStatus::Passed
        } else {
            CheckStatus::Failed
        },
        installed,
        install_location,
    }
}

fn default_home_path(relative_path: &str) -> Option<PathBuf> {
    env::var_os("HOME").map(|home| PathBuf::from(home).join(relative_path))
}

fn merge_checks(
    base: &mut BTreeMap<String, ReadinessCheck>,
    additional: BTreeMap<String, ReadinessCheck>,
) {
    for (name, check) in additional {
        base.insert(name, check);
    }
}

fn determine_mode(checks: &BTreeMap<String, ReadinessCheck>) -> OperatingMode {
    let log_backed_ready = passed(checks, "log_source_reachable")
        && passed(checks, "statement_evidence")
        && passed(checks, "duration_evidence")
        && passed(checks, "correlation_evidence");
    if log_backed_ready {
        return OperatingMode::LogBacked;
    }

    let live_only_ready = passed(checks, "track_activities")
        && passed(checks, "shared_preload_libraries")
        && passed(checks, "compute_query_id")
        && passed(checks, "pg_stat_statements_extension")
        && passed(checks, "pg_read_all_stats")
        && passed(checks, "pg_stat_activity_probe")
        && passed(checks, "pg_stat_statements_probe");

    if live_only_ready {
        OperatingMode::LiveOnly
    } else {
        OperatingMode::Unready
    }
}

fn collect_failed_checks(checks: &BTreeMap<String, ReadinessCheck>) -> Vec<String> {
    checks
        .iter()
        .filter(|(_, check)| matches!(check.status, CheckStatus::Failed))
        .map(|(name, check)| check.reason.clone().unwrap_or_else(|| name.to_string()))
        .collect()
}

fn build_limitations(
    mode: OperatingMode,
    checks: &BTreeMap<String, ReadinessCheck>,
) -> Vec<String> {
    match mode {
        OperatingMode::LogBacked => {
            let mut limitations = Vec::new();
            if checks
                .get("pg_stat_activity_probe")
                .is_some_and(|check| matches!(check.status, CheckStatus::Skipped))
            {
                limitations.push("live_database_checks_unavailable".to_string());
            }
            limitations
        }
        OperatingMode::LiveOnly => vec![
            "historical_log_triage_unavailable".to_string(),
            "event_level_evidence_unavailable".to_string(),
        ],
        OperatingMode::Unready => {
            let mut limitations = vec!["supported_evidence_unavailable".to_string()];
            if checks
                .get("pg_stat_activity_probe")
                .is_some_and(|check| matches!(check.status, CheckStatus::Skipped))
            {
                limitations.push("database_connection_not_configured".to_string());
            }
            limitations
        }
    }
}

fn recommended_next_commands(mode: OperatingMode) -> Vec<String> {
    match mode {
        OperatingMode::LogBacked => {
            vec!["pg-logstats top query-families --output-format json".to_string()]
        }
        OperatingMode::LiveOnly => {
            vec!["pg-logstats running-queries --output-format json".to_string()]
        }
        OperatingMode::Unready => vec![
            format!("pg-logstats readiness --output-format json --dsn <postgres-url>"),
            format!("pg-logstats readiness --output-format json <log-file>"),
        ],
    }
}

fn passed(checks: &BTreeMap<String, ReadinessCheck>, name: &str) -> bool {
    checks
        .get(name)
        .is_some_and(|check| matches!(check.status, CheckStatus::Passed))
}

fn connect_client(dsn: &str, connect_timeout_ms: Option<u64>) -> Result<Client, String> {
    let mut config = postgres::Config::from_str(dsn)
        .map_err(|err| format!("database_connection_invalid: {err}"))?;
    if let Some(connect_timeout_ms) = connect_timeout_ms {
        config.connect_timeout(Duration::from_millis(connect_timeout_ms));
    }
    config
        .connect(NoTls)
        .map_err(|err| format!("database_connection_failed: {err}"))
}

fn show_setting(client: &mut Client, setting: &str) -> Option<String> {
    let sql = format!("SHOW {setting}");
    client
        .query_one(&sql, &[])
        .ok()
        .and_then(|row| row.try_get::<_, String>(0).ok())
}

fn evaluate_log_destination(value: Option<&str>) -> ReadinessCheck {
    let Some(value) = value else {
        return failed_check(None, Some("log_destination_unavailable"));
    };

    let destinations = split_csv_setting(value);
    if destinations.iter().any(|item| item == "stderr") {
        passed_check(Some(json!(value)), None)
    } else if destinations
        .iter()
        .any(|item| item == "csvlog" || item == "jsonlog")
    {
        failed_check(Some(json!(value)), Some("unsupported_log_format"))
    } else {
        failed_check(Some(json!(value)), Some("unsupported_log_destination"))
    }
}

fn evaluate_log_line_prefix(value: Option<&str>) -> ReadinessCheck {
    let Some(value) = value else {
        return failed_check(None, Some("log_line_prefix_unavailable"));
    };

    if value.contains("%p") {
        passed_check(Some(json!(value)), None)
    } else {
        failed_check(
            Some(json!(value)),
            Some("log_line_prefix_missing_process_id"),
        )
    }
}

fn evaluate_on_off_setting(value: Option<&str>, failure_reason: &str) -> ReadinessCheck {
    let Some(value) = value else {
        return failed_check(None, Some(failure_reason));
    };

    if value.eq_ignore_ascii_case("on") {
        passed_check(Some(json!(value)), None)
    } else {
        failed_check(Some(json!(value)), Some(failure_reason))
    }
}

fn evaluate_non_negative_setting(value: Option<&str>, failure_reason: &str) -> ReadinessCheck {
    let Some(value) = value else {
        return failed_check(None, Some(failure_reason));
    };

    if value.parse::<i64>().ok().is_some_and(|parsed| parsed >= 0) {
        passed_check(Some(json!(value)), None)
    } else {
        failed_check(Some(json!(value)), Some(failure_reason))
    }
}

fn evaluate_list_contains(
    value: Option<&str>,
    required: &str,
    failure_reason: &str,
) -> ReadinessCheck {
    let Some(value) = value else {
        return failed_check(None, Some(failure_reason));
    };

    if split_csv_setting(value).iter().any(|item| item == required) {
        passed_check(Some(json!(value)), None)
    } else {
        failed_check(Some(json!(value)), Some(failure_reason))
    }
}

fn evaluate_allowed_values(
    value: Option<&str>,
    allowed: &[&str],
    failure_reason: &str,
) -> ReadinessCheck {
    let Some(value) = value else {
        return failed_check(None, Some(failure_reason));
    };

    if allowed
        .iter()
        .any(|candidate| value.eq_ignore_ascii_case(candidate))
    {
        passed_check(Some(json!(value)), None)
    } else {
        failed_check(Some(json!(value)), Some(failure_reason))
    }
}

fn evaluate_exists_query(client: &mut Client, sql: &str, failure_reason: &str) -> ReadinessCheck {
    match client.query_one(sql, &[]) {
        Ok(row) => match row.try_get::<_, bool>(0) {
            Ok(true) => passed_check(Some(json!(true)), None),
            Ok(false) => failed_check(Some(json!(false)), Some(failure_reason)),
            Err(err) => failed_check(None, Some(&format!("{failure_reason}: {err}"))),
        },
        Err(err) => failed_check(None, Some(&format!("{failure_reason}: {err}"))),
    }
}

fn evaluate_probe_query(client: &mut Client, sql: &str) -> ReadinessCheck {
    match client.query(sql, &[]) {
        Ok(_) => passed_check(None, None),
        Err(err) => failed_check(None, Some(&format!("probe_failed: {err}"))),
    }
}

fn split_csv_setting(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|item| item.trim().to_string())
        .collect()
}

fn passed_check(value: Option<Value>, reason: Option<&str>) -> ReadinessCheck {
    ReadinessCheck {
        status: CheckStatus::Passed,
        value,
        reason: reason.map(str::to_string),
    }
}

fn failed_check(value: Option<Value>, reason: Option<&str>) -> ReadinessCheck {
    ReadinessCheck {
        status: CheckStatus::Failed,
        value,
        reason: reason.map(str::to_string),
    }
}

fn skipped_check(reason: Option<&str>, value: Option<Value>) -> ReadinessCheck {
    ReadinessCheck {
        status: CheckStatus::Skipped,
        value,
        reason: reason.map(str::to_string),
    }
}

fn check_status_label(status: CheckStatus) -> &'static str {
    match status {
        CheckStatus::Passed => "passed",
        CheckStatus::Failed => "failed",
        CheckStatus::Skipped => "skipped",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LogLevel;
    use chrono::{TimeZone, Utc};
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn sample_log_evidence() -> LogEvidence {
        LogEvidence {
            source_kind: EventSourceKind::Stderr,
            entries: vec![
                LogEntry::new(
                    Utc.with_ymd_and_hms(2024, 1, 15, 10, 0, 0).unwrap(),
                    "1001".to_string(),
                    LogLevel::Statement,
                    "statement: SELECT 1".to_string(),
                ),
                LogEntry::new(
                    Utc.with_ymd_and_hms(2024, 1, 15, 10, 0, 1).unwrap(),
                    "1001".to_string(),
                    LogLevel::Duration,
                    "duration: 10 ms".to_string(),
                ),
            ],
        }
    }

    #[test]
    fn resolves_database_dsn_precedence() {
        let _guard = env_lock().lock().unwrap();
        let config = AppConfig::default();
        env::set_var(DATABASE_URL_ENV_VAR, "postgres://env");
        assert_eq!(
            resolve_database_dsn(Some("postgres://explicit"), &config).as_deref(),
            Some("postgres://explicit")
        );
        env::remove_var(DATABASE_URL_ENV_VAR);
    }

    #[test]
    fn readiness_uses_log_evidence_without_database_connection() {
        let report = build_readiness_report(
            &AppConfig::default(),
            None,
            LogReadinessEvidence::Available(sample_log_evidence()),
        );

        assert_eq!(report.operating_mode, OperatingMode::LogBacked);
        assert_eq!(
            report.payload.readiness.database_readiness.checks["pg_stat_activity_probe"].status,
            CheckStatus::Skipped
        );
    }

    #[test]
    fn readiness_without_any_evidence_is_unready() {
        let report = build_readiness_report(
            &AppConfig::default(),
            None,
            LogReadinessEvidence::NotRequested,
        );

        assert_eq!(report.operating_mode, OperatingMode::Unready);
        assert!(report.payload.readiness.failed_checks.is_empty());
        assert!(report
            .limitations
            .contains(&"database_connection_not_configured".to_string()));
    }

    #[test]
    fn unreachable_log_source_records_failed_check() {
        let report = build_readiness_report(
            &AppConfig::default(),
            None,
            LogReadinessEvidence::Unreachable {
                reason: "supported_log_source_unreachable".to_string(),
            },
        );

        assert_eq!(
            report.payload.readiness.database_readiness.checks["log_source_reachable"].status,
            CheckStatus::Failed
        );
    }
}
