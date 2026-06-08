use crate::config::{AgentInstallTargetConfig, AppConfig};
use crate::database::connect_postgres_client;
use crate::triage::{
    CheckStatus, OperatingMode, PgTriageReport, WorkflowId, PG_TRIAGE_SCHEMA_VERSION,
};
use crate::{normalize_log_entries, Correlator, EventSourceKind, LogEntry, ProcessOrderCorrelator};
use postgres::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};

const REQUIRED_CHECKS: &[ReadinessCheckId] = &[
    ReadinessCheckId::LogSourceReachable,
    ReadinessCheckId::StatementEvidence,
    ReadinessCheckId::DurationEvidence,
    ReadinessCheckId::CorrelationEvidence,
    ReadinessCheckId::LogDestination,
    ReadinessCheckId::LogLinePrefix,
    ReadinessCheckId::LogDuration,
    ReadinessCheckId::LogMinDurationStatement,
    ReadinessCheckId::LogTempFiles,
    ReadinessCheckId::TrackActivities,
    ReadinessCheckId::SharedPreloadLibraries,
    ReadinessCheckId::ComputeQueryId,
    ReadinessCheckId::PgStatStatementsExtension,
    ReadinessCheckId::PgReadAllStats,
    ReadinessCheckId::PgStatActivityProbe,
    ReadinessCheckId::PgStatStatementsProbe,
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessReportPayload {
    pub readiness: ReadinessDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadinessDetails {
    pub database_readiness: DatabaseReadiness,
    pub agent_readiness: AgentReadiness,
    pub required_checks: Vec<ReadinessCheckId>,
    pub failed_checks: Vec<ReadinessReason>,
    pub recommended_next_commands: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseReadiness {
    pub mode_candidate: OperatingMode,
    pub checks: BTreeMap<ReadinessCheckId, ReadinessCheck>,
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
    pub reason: Option<ReadinessReason>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessCheckId {
    LogSourceReachable,
    StatementEvidence,
    DurationEvidence,
    CorrelationEvidence,
    LogDestination,
    LogLinePrefix,
    LogDuration,
    LogMinDurationStatement,
    LogTempFiles,
    TrackActivities,
    SharedPreloadLibraries,
    ComputeQueryId,
    PgStatStatementsExtension,
    PgReadAllStats,
    PgStatActivityProbe,
    PgStatStatementsProbe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReadinessReason {
    LogSourceNotRequested,
    LogSourceUnreachable,
    SupportedLogSourceUnreachable,
    DatabaseConnectionNotConfigured,
    DatabaseConnectionInvalid,
    DatabaseConnectionFailed,
    LogDestinationUnavailable,
    UnsupportedLogFormat,
    UnsupportedLogDestination,
    LogLinePrefixUnavailable,
    LogLinePrefixMissingProcessId,
    LogDurationDisabled,
    LogMinDurationStatementDisabled,
    LogTempFilesDisabled,
    TrackActivitiesDisabled,
    PgStatStatementsNotPreloaded,
    ComputeQueryIdDisabled,
    PgStatStatementsExtensionMissing,
    PgReadAllStatsUnavailable,
    StatementEvidenceMissing,
    DurationEvidenceMissing,
    CorrelationEvidenceMissing,
    ProbeFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadinessLimitation {
    LiveDatabaseChecksUnavailable,
    HistoricalLogTriageUnavailable,
    EventLevelEvidenceUnavailable,
    SupportedEvidenceUnavailable,
    DatabaseConnectionNotConfigured,
}

impl ReadinessLimitation {
    fn as_str(self) -> &'static str {
        match self {
            Self::LiveDatabaseChecksUnavailable => "live_database_checks_unavailable",
            Self::HistoricalLogTriageUnavailable => "historical_log_triage_unavailable",
            Self::EventLevelEvidenceUnavailable => "event_level_evidence_unavailable",
            Self::SupportedEvidenceUnavailable => "supported_evidence_unavailable",
            Self::DatabaseConnectionNotConfigured => "database_connection_not_configured",
        }
    }
}

#[derive(Debug, Clone)]
pub struct LogEvidence {
    pub entries: Vec<LogEntry>,
    pub source_kind: EventSourceKind,
}

#[derive(Debug, Clone)]
pub enum LogReadinessEvidence {
    NotRequested,
    Unreachable { reason: ReadinessReason },
    Available(LogEvidence),
}

pub fn build_readiness_report(
    config: &AppConfig,
    resolved_dsn: Option<&str>,
    log_evidence: LogReadinessEvidence,
) -> PgTriageReport<ReadinessReportPayload> {
    let mut checks = build_log_checks(log_evidence);
    merge_checks(
        &mut checks,
        build_database_checks(resolved_dsn.map(str::to_string), config),
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
                required_checks: REQUIRED_CHECKS.to_vec(),
                failed_checks,
                recommended_next_commands,
            },
        },
    }
}

pub fn format_readiness_text(report: &PgTriageReport<ReadinessReportPayload>) -> String {
    let readiness = &report.payload.readiness;
    let mut output = String::new();
    output.push_str(&format!(
        "Operating Mode: {}\n",
        operating_mode_label(report.operating_mode)
    ));

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
        output.push_str(&format!(
            "- {}: {}\n",
            check_id_label(*name),
            check_status_label(check.status)
        ));
        if let Some(value) = &check.value {
            output.push_str(&format!("  value: {value}\n"));
        }
        if let Some(reason) = &check.reason {
            output.push_str(&format!("  reason: {}\n", reason_label(*reason)));
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

fn build_log_checks(
    log_evidence: LogReadinessEvidence,
) -> BTreeMap<ReadinessCheckId, ReadinessCheck> {
    let mut checks = BTreeMap::new();
    let log_evidence = match log_evidence {
        LogReadinessEvidence::NotRequested => {
            for name in [
                ReadinessCheckId::LogSourceReachable,
                ReadinessCheckId::StatementEvidence,
                ReadinessCheckId::DurationEvidence,
                ReadinessCheckId::CorrelationEvidence,
            ] {
                checks.insert(
                    name,
                    skipped_check(Some(ReadinessReason::LogSourceNotRequested), None),
                );
            }
            return checks;
        }
        LogReadinessEvidence::Unreachable { reason } => {
            checks.insert(
                ReadinessCheckId::LogSourceReachable,
                failed_check(None, Some(reason)),
            );
            for name in [
                ReadinessCheckId::StatementEvidence,
                ReadinessCheckId::DurationEvidence,
                ReadinessCheckId::CorrelationEvidence,
            ] {
                checks.insert(
                    name,
                    skipped_check(Some(ReadinessReason::LogSourceUnreachable), None),
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
        ReadinessCheckId::LogSourceReachable,
        passed_check(Some(json!(entries.len())), None),
    );
    checks.insert(
        ReadinessCheckId::StatementEvidence,
        if has_statement {
            passed_check(Some(json!(true)), None)
        } else {
            failed_check(
                Some(json!(false)),
                Some(ReadinessReason::StatementEvidenceMissing),
            )
        },
    );
    checks.insert(
        ReadinessCheckId::DurationEvidence,
        if has_duration {
            passed_check(Some(json!(true)), None)
        } else {
            failed_check(
                Some(json!(false)),
                Some(ReadinessReason::DurationEvidenceMissing),
            )
        },
    );
    checks.insert(
        ReadinessCheckId::CorrelationEvidence,
        if has_correlated_execution {
            passed_check(Some(json!(true)), None)
        } else {
            failed_check(
                Some(json!(false)),
                Some(ReadinessReason::CorrelationEvidenceMissing),
            )
        },
    );

    checks
}

fn build_database_checks(
    dsn: Option<String>,
    config: &AppConfig,
) -> BTreeMap<ReadinessCheckId, ReadinessCheck> {
    let mut checks = BTreeMap::new();
    let Some(dsn) = dsn else {
        for name in [
            ReadinessCheckId::LogDestination,
            ReadinessCheckId::LogLinePrefix,
            ReadinessCheckId::LogDuration,
            ReadinessCheckId::LogMinDurationStatement,
            ReadinessCheckId::LogTempFiles,
            ReadinessCheckId::TrackActivities,
            ReadinessCheckId::SharedPreloadLibraries,
            ReadinessCheckId::ComputeQueryId,
            ReadinessCheckId::PgStatStatementsExtension,
            ReadinessCheckId::PgReadAllStats,
            ReadinessCheckId::PgStatActivityProbe,
            ReadinessCheckId::PgStatStatementsProbe,
        ] {
            checks.insert(
                name,
                skipped_check(Some(ReadinessReason::DatabaseConnectionNotConfigured), None),
            );
        }
        return checks;
    };

    let mut client = match connect_postgres_client(&dsn, config.database.connect_timeout_ms) {
        Ok(client) => client,
        Err(reason) => {
            for name in [
                ReadinessCheckId::LogDestination,
                ReadinessCheckId::LogLinePrefix,
                ReadinessCheckId::LogDuration,
                ReadinessCheckId::LogMinDurationStatement,
                ReadinessCheckId::LogTempFiles,
                ReadinessCheckId::TrackActivities,
                ReadinessCheckId::SharedPreloadLibraries,
                ReadinessCheckId::ComputeQueryId,
                ReadinessCheckId::PgStatStatementsExtension,
                ReadinessCheckId::PgReadAllStats,
                ReadinessCheckId::PgStatActivityProbe,
                ReadinessCheckId::PgStatStatementsProbe,
            ] {
                checks.insert(
                    name,
                    failed_check(None, Some(parse_connection_reason(&reason))),
                );
            }
            return checks;
        }
    };

    checks.insert(
        ReadinessCheckId::LogDestination,
        evaluate_log_destination(show_setting(&mut client, "log_destination").as_deref()),
    );
    checks.insert(
        ReadinessCheckId::LogLinePrefix,
        evaluate_log_line_prefix(show_setting(&mut client, "log_line_prefix").as_deref()),
    );
    checks.insert(
        ReadinessCheckId::LogDuration,
        evaluate_on_off_setting(
            show_setting(&mut client, "log_duration").as_deref(),
            ReadinessReason::LogDurationDisabled,
        ),
    );
    checks.insert(
        ReadinessCheckId::LogMinDurationStatement,
        evaluate_non_negative_setting(
            show_setting(&mut client, "log_min_duration_statement").as_deref(),
            ReadinessReason::LogMinDurationStatementDisabled,
        ),
    );
    checks.insert(
        ReadinessCheckId::LogTempFiles,
        evaluate_non_negative_setting(
            show_setting(&mut client, "log_temp_files").as_deref(),
            ReadinessReason::LogTempFilesDisabled,
        ),
    );
    checks.insert(
        ReadinessCheckId::TrackActivities,
        evaluate_on_off_setting(
            show_setting(&mut client, "track_activities").as_deref(),
            ReadinessReason::TrackActivitiesDisabled,
        ),
    );
    checks.insert(
        ReadinessCheckId::SharedPreloadLibraries,
        evaluate_list_contains(
            show_setting(&mut client, "shared_preload_libraries").as_deref(),
            "pg_stat_statements",
            ReadinessReason::PgStatStatementsNotPreloaded,
        ),
    );
    checks.insert(
        ReadinessCheckId::ComputeQueryId,
        evaluate_allowed_values(
            show_setting(&mut client, "compute_query_id").as_deref(),
            &["auto", "on"],
            ReadinessReason::ComputeQueryIdDisabled,
        ),
    );
    checks.insert(
        ReadinessCheckId::PgStatStatementsExtension,
        evaluate_exists_query(
            &mut client,
            "SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'pg_stat_statements')",
            ReadinessReason::PgStatStatementsExtensionMissing,
        ),
    );
    checks.insert(
        ReadinessCheckId::PgReadAllStats,
        evaluate_exists_query(
            &mut client,
            "SELECT pg_has_role(current_user, 'pg_read_all_stats', 'member') OR (SELECT rolsuper FROM pg_roles WHERE rolname = current_user)",
            ReadinessReason::PgReadAllStatsUnavailable,
        ),
    );
    checks.insert(
        ReadinessCheckId::PgStatActivityProbe,
        evaluate_probe_query(
            &mut client,
            "SELECT pid, datname, usename, application_name, state, wait_event_type, wait_event, query_id, query FROM pg_stat_activity LIMIT 1",
        ),
    );
    checks.insert(
        ReadinessCheckId::PgStatStatementsProbe,
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
    base: &mut BTreeMap<ReadinessCheckId, ReadinessCheck>,
    additional: BTreeMap<ReadinessCheckId, ReadinessCheck>,
) {
    for (name, check) in additional {
        base.insert(name, check);
    }
}

fn determine_mode(checks: &BTreeMap<ReadinessCheckId, ReadinessCheck>) -> OperatingMode {
    let log_backed_ready = passed(checks, ReadinessCheckId::LogSourceReachable)
        && passed(checks, ReadinessCheckId::StatementEvidence)
        && passed(checks, ReadinessCheckId::DurationEvidence)
        && passed(checks, ReadinessCheckId::CorrelationEvidence);
    if log_backed_ready {
        return OperatingMode::LogBacked;
    }

    let live_only_ready = passed(checks, ReadinessCheckId::TrackActivities)
        && passed(checks, ReadinessCheckId::SharedPreloadLibraries)
        && passed(checks, ReadinessCheckId::ComputeQueryId)
        && passed(checks, ReadinessCheckId::PgStatStatementsExtension)
        && passed(checks, ReadinessCheckId::PgReadAllStats)
        && passed(checks, ReadinessCheckId::PgStatActivityProbe)
        && passed(checks, ReadinessCheckId::PgStatStatementsProbe);

    if live_only_ready {
        OperatingMode::LiveOnly
    } else {
        OperatingMode::Unready
    }
}

fn collect_failed_checks(
    checks: &BTreeMap<ReadinessCheckId, ReadinessCheck>,
) -> Vec<ReadinessReason> {
    checks
        .values()
        .filter(|check| matches!(check.status, CheckStatus::Failed))
        .filter_map(|check| check.reason)
        .collect()
}

fn build_limitations(
    mode: OperatingMode,
    checks: &BTreeMap<ReadinessCheckId, ReadinessCheck>,
) -> Vec<String> {
    match mode {
        OperatingMode::LogBacked => {
            let mut limitations = Vec::new();
            if checks
                .get(&ReadinessCheckId::PgStatActivityProbe)
                .is_some_and(|check| matches!(check.status, CheckStatus::Skipped))
            {
                limitations.push(
                    ReadinessLimitation::LiveDatabaseChecksUnavailable
                        .as_str()
                        .to_string(),
                );
            }
            limitations
        }
        OperatingMode::LiveOnly => vec![
            ReadinessLimitation::HistoricalLogTriageUnavailable
                .as_str()
                .to_string(),
            ReadinessLimitation::EventLevelEvidenceUnavailable
                .as_str()
                .to_string(),
        ],
        OperatingMode::Unready => {
            let mut limitations = vec![ReadinessLimitation::SupportedEvidenceUnavailable
                .as_str()
                .to_string()];
            if checks
                .get(&ReadinessCheckId::PgStatActivityProbe)
                .is_some_and(|check| matches!(check.status, CheckStatus::Skipped))
            {
                limitations.push(
                    ReadinessLimitation::DatabaseConnectionNotConfigured
                        .as_str()
                        .to_string(),
                );
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
            "pg-logstats readiness --output-format json --dsn <postgres-url>".to_string(),
            "pg-logstats readiness --output-format json <log-file>".to_string(),
        ],
    }
}

fn passed(checks: &BTreeMap<ReadinessCheckId, ReadinessCheck>, name: ReadinessCheckId) -> bool {
    checks
        .get(&name)
        .is_some_and(|check| matches!(check.status, CheckStatus::Passed))
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
        return failed_check(None, Some(ReadinessReason::LogDestinationUnavailable));
    };

    let destinations = split_csv_setting(value);
    if destinations.iter().any(|item| item == "stderr") {
        passed_check(Some(json!(value)), None)
    } else if destinations
        .iter()
        .any(|item| item == "csvlog" || item == "jsonlog")
    {
        failed_check(
            Some(json!(value)),
            Some(ReadinessReason::UnsupportedLogFormat),
        )
    } else {
        failed_check(
            Some(json!(value)),
            Some(ReadinessReason::UnsupportedLogDestination),
        )
    }
}

fn evaluate_log_line_prefix(value: Option<&str>) -> ReadinessCheck {
    let Some(value) = value else {
        return failed_check(None, Some(ReadinessReason::LogLinePrefixUnavailable));
    };

    if value.contains("%p") {
        passed_check(Some(json!(value)), None)
    } else {
        failed_check(
            Some(json!(value)),
            Some(ReadinessReason::LogLinePrefixMissingProcessId),
        )
    }
}

fn evaluate_on_off_setting(value: Option<&str>, failure_reason: ReadinessReason) -> ReadinessCheck {
    let Some(value) = value else {
        return failed_check(None, Some(failure_reason));
    };

    if value.eq_ignore_ascii_case("on") {
        passed_check(Some(json!(value)), None)
    } else {
        failed_check(Some(json!(value)), Some(failure_reason))
    }
}

fn evaluate_non_negative_setting(
    value: Option<&str>,
    failure_reason: ReadinessReason,
) -> ReadinessCheck {
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
    failure_reason: ReadinessReason,
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
    failure_reason: ReadinessReason,
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

fn evaluate_exists_query(
    client: &mut Client,
    sql: &str,
    failure_reason: ReadinessReason,
) -> ReadinessCheck {
    match client.query_one(sql, &[]) {
        Ok(row) => match row.try_get::<_, bool>(0) {
            Ok(true) => passed_check(Some(json!(true)), None),
            Ok(false) => failed_check(Some(json!(false)), Some(failure_reason)),
            Err(_) => failed_check(None, Some(ReadinessReason::ProbeFailed)),
        },
        Err(_) => failed_check(None, Some(ReadinessReason::ProbeFailed)),
    }
}

fn evaluate_probe_query(client: &mut Client, sql: &str) -> ReadinessCheck {
    match client.query(sql, &[]) {
        Ok(_) => passed_check(None, None),
        Err(_) => failed_check(None, Some(ReadinessReason::ProbeFailed)),
    }
}

fn split_csv_setting(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|item| item.trim().to_string())
        .collect()
}

fn passed_check(value: Option<Value>, reason: Option<ReadinessReason>) -> ReadinessCheck {
    ReadinessCheck {
        status: CheckStatus::Passed,
        value,
        reason,
    }
}

fn failed_check(value: Option<Value>, reason: Option<ReadinessReason>) -> ReadinessCheck {
    ReadinessCheck {
        status: CheckStatus::Failed,
        value,
        reason,
    }
}

fn skipped_check(reason: Option<ReadinessReason>, value: Option<Value>) -> ReadinessCheck {
    ReadinessCheck {
        status: CheckStatus::Skipped,
        value,
        reason,
    }
}

fn parse_connection_reason(reason: &str) -> ReadinessReason {
    if reason.starts_with("database_connection_invalid") {
        ReadinessReason::DatabaseConnectionInvalid
    } else {
        ReadinessReason::DatabaseConnectionFailed
    }
}

fn check_status_label(status: CheckStatus) -> &'static str {
    match status {
        CheckStatus::Passed => "passed",
        CheckStatus::Failed => "failed",
        CheckStatus::Skipped => "skipped",
    }
}

fn operating_mode_label(mode: OperatingMode) -> &'static str {
    match mode {
        OperatingMode::LogBacked => "log_backed",
        OperatingMode::LiveOnly => "live_only",
        OperatingMode::Unready => "unready",
    }
}

fn check_id_label(id: ReadinessCheckId) -> &'static str {
    match id {
        ReadinessCheckId::LogSourceReachable => "log_source_reachable",
        ReadinessCheckId::StatementEvidence => "statement_evidence",
        ReadinessCheckId::DurationEvidence => "duration_evidence",
        ReadinessCheckId::CorrelationEvidence => "correlation_evidence",
        ReadinessCheckId::LogDestination => "log_destination",
        ReadinessCheckId::LogLinePrefix => "log_line_prefix",
        ReadinessCheckId::LogDuration => "log_duration",
        ReadinessCheckId::LogMinDurationStatement => "log_min_duration_statement",
        ReadinessCheckId::LogTempFiles => "log_temp_files",
        ReadinessCheckId::TrackActivities => "track_activities",
        ReadinessCheckId::SharedPreloadLibraries => "shared_preload_libraries",
        ReadinessCheckId::ComputeQueryId => "compute_query_id",
        ReadinessCheckId::PgStatStatementsExtension => "pg_stat_statements_extension",
        ReadinessCheckId::PgReadAllStats => "pg_read_all_stats",
        ReadinessCheckId::PgStatActivityProbe => "pg_stat_activity_probe",
        ReadinessCheckId::PgStatStatementsProbe => "pg_stat_statements_probe",
    }
}

fn reason_label(reason: ReadinessReason) -> &'static str {
    match reason {
        ReadinessReason::LogSourceNotRequested => "log_source_not_requested",
        ReadinessReason::LogSourceUnreachable => "log_source_unreachable",
        ReadinessReason::SupportedLogSourceUnreachable => "supported_log_source_unreachable",
        ReadinessReason::DatabaseConnectionNotConfigured => "database_connection_not_configured",
        ReadinessReason::DatabaseConnectionInvalid => "database_connection_invalid",
        ReadinessReason::DatabaseConnectionFailed => "database_connection_failed",
        ReadinessReason::LogDestinationUnavailable => "log_destination_unavailable",
        ReadinessReason::UnsupportedLogFormat => "unsupported_log_format",
        ReadinessReason::UnsupportedLogDestination => "unsupported_log_destination",
        ReadinessReason::LogLinePrefixUnavailable => "log_line_prefix_unavailable",
        ReadinessReason::LogLinePrefixMissingProcessId => "log_line_prefix_missing_process_id",
        ReadinessReason::LogDurationDisabled => "log_duration_disabled",
        ReadinessReason::LogMinDurationStatementDisabled => "log_min_duration_statement_disabled",
        ReadinessReason::LogTempFilesDisabled => "log_temp_files_disabled",
        ReadinessReason::TrackActivitiesDisabled => "track_activities_disabled",
        ReadinessReason::PgStatStatementsNotPreloaded => "pg_stat_statements_not_preloaded",
        ReadinessReason::ComputeQueryIdDisabled => "compute_query_id_disabled",
        ReadinessReason::PgStatStatementsExtensionMissing => "pg_stat_statements_extension_missing",
        ReadinessReason::PgReadAllStatsUnavailable => "pg_read_all_stats_unavailable",
        ReadinessReason::StatementEvidenceMissing => "statement_evidence_missing",
        ReadinessReason::DurationEvidenceMissing => "duration_evidence_missing",
        ReadinessReason::CorrelationEvidenceMissing => "correlation_evidence_missing",
        ReadinessReason::ProbeFailed => "probe_failed",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LogLevel;
    use chrono::{TimeZone, Utc};

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
    fn readiness_uses_log_evidence_without_database_connection() {
        let report = build_readiness_report(
            &AppConfig::default(),
            None,
            LogReadinessEvidence::Available(sample_log_evidence()),
        );

        assert_eq!(report.operating_mode, OperatingMode::LogBacked);
        assert_eq!(
            report.payload.readiness.database_readiness.checks
                [&ReadinessCheckId::PgStatActivityProbe]
                .status,
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
        assert!(report.limitations.contains(
            &ReadinessLimitation::DatabaseConnectionNotConfigured
                .as_str()
                .to_string()
        ));
    }

    #[test]
    fn unreachable_log_source_records_failed_check() {
        let report = build_readiness_report(
            &AppConfig::default(),
            None,
            LogReadinessEvidence::Unreachable {
                reason: ReadinessReason::SupportedLogSourceUnreachable,
            },
        );

        assert_eq!(
            report.payload.readiness.database_readiness.checks
                [&ReadinessCheckId::LogSourceReachable]
                .status,
            CheckStatus::Failed
        );
    }
}
