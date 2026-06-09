use crate::config::{AgentInstallTargetConfig, AppConfig};
use crate::database::connect_postgres_client;
use crate::input::{
    discover_log_files, process_cloudwatch_input, process_log_file, CloudWatchInput, LocalLogInput,
};
use crate::triage::{
    CheckStatus, OperatingMode, PgTriageReport, WorkflowId, PG_TRIAGE_SCHEMA_VERSION,
};
use crate::{
    normalize_log_entries, Correlator, EventSourceKind, LogEntry, ProcessOrderCorrelator,
    TextLogParser,
};
use postgres::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::env;
use std::path::{Path, PathBuf};

const REQUIRED_CHECKS: &[InspectCheckId] = &[
    InspectCheckId::LogSourceReachable,
    InspectCheckId::StatementEvidence,
    InspectCheckId::DurationEvidence,
    InspectCheckId::CorrelationEvidence,
    InspectCheckId::LogDestination,
    InspectCheckId::LogLinePrefix,
    InspectCheckId::LogDuration,
    InspectCheckId::LogMinDurationStatement,
    InspectCheckId::LogTempFiles,
    InspectCheckId::TrackActivities,
    InspectCheckId::SharedPreloadLibraries,
    InspectCheckId::ComputeQueryId,
    InspectCheckId::PgStatStatementsExtension,
    InspectCheckId::PgReadAllStats,
    InspectCheckId::PgStatActivityProbe,
    InspectCheckId::PgStatStatementsProbe,
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectReportPayload {
    pub inspect: InspectDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectDetails {
    pub database_inspect: DatabaseInspect,
    pub agent_inspect: AgentInspect,
    pub required_checks: Vec<InspectCheckId>,
    pub failed_checks: Vec<InspectReason>,
    pub recommended_next_commands: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseInspect {
    pub mode_candidate: OperatingMode,
    pub checks: BTreeMap<InspectCheckId, InspectCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInspect {
    pub codex: AgentTargetInspect,
    pub claude: AgentTargetInspect,
    pub gemini: AgentTargetInspect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTargetInspect {
    pub status: CheckStatus,
    pub installed: bool,
    pub install_location: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectCheck {
    pub status: CheckStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<InspectReason>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InspectCheckId {
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
pub enum InspectReason {
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
pub enum InspectLimitation {
    LiveDatabaseChecksUnavailable,
    HistoricalLogTriageUnavailable,
    EventLevelEvidenceUnavailable,
    SupportedEvidenceUnavailable,
    DatabaseConnectionNotConfigured,
}

impl InspectLimitation {
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
pub enum LogInspectEvidence {
    NotRequested,
    Unreachable { reason: InspectReason },
    Available(LogEvidence),
}

pub fn build_inspect_report(
    config: &AppConfig,
    resolved_dsn: Option<&str>,
    log_evidence: LogInspectEvidence,
) -> PgTriageReport<InspectReportPayload> {
    let mut checks = build_log_checks(log_evidence);
    merge_checks(
        &mut checks,
        build_database_checks(resolved_dsn.map(str::to_string), config),
    );

    let mode_candidate = determine_mode(&checks);
    let failed_checks = collect_failed_checks(&checks);
    let limitations = build_limitations(mode_candidate, &checks);
    let recommended_next_commands = recommended_next_commands(mode_candidate);
    let agent_inspect = detect_agent_inspect(config);

    PgTriageReport {
        schema_version: PG_TRIAGE_SCHEMA_VERSION,
        workflow: WorkflowId::Inspect,
        operating_mode: mode_candidate,
        limitations,
        verdict: None,
        verdict_reasons: Vec::new(),
        allowed_actions: None,
        blocked_actions: None,
        analysis_window: None,
        source_summary: None,
        payload: InspectReportPayload {
            inspect: InspectDetails {
                database_inspect: DatabaseInspect {
                    mode_candidate,
                    checks,
                },
                agent_inspect,
                required_checks: REQUIRED_CHECKS.to_vec(),
                failed_checks,
                recommended_next_commands,
            },
        },
    }
}

pub fn collect_log_inspect_evidence(
    local_input: &LocalLogInput,
    cloudwatch_input: Option<&CloudWatchInput>,
    parser: &TextLogParser,
    source_kind: EventSourceKind,
) -> LogInspectEvidence {
    if let Some(cloudwatch_input) = cloudwatch_input {
        return match process_cloudwatch_input(cloudwatch_input, parser) {
            Ok(entries) if !entries.is_empty() => LogInspectEvidence::Available(LogEvidence {
                entries,
                source_kind,
            }),
            Ok(_) => LogInspectEvidence::Unreachable {
                reason: InspectReason::SupportedLogSourceUnreachable,
            },
            Err(_) => LogInspectEvidence::Unreachable {
                reason: InspectReason::SupportedLogSourceUnreachable,
            },
        };
    }

    let log_files = match discover_log_files(local_input) {
        Ok(log_files) => log_files,
        Err(_) => {
            return if local_input.log_dir.is_some()
                || local_input.logfile_list.is_some()
                || !local_input.log_files.is_empty()
            {
                LogInspectEvidence::Unreachable {
                    reason: InspectReason::SupportedLogSourceUnreachable,
                }
            } else {
                LogInspectEvidence::NotRequested
            };
        }
    };

    if log_files.is_empty() {
        return if local_input.log_dir.is_some()
            || local_input.logfile_list.is_some()
            || !local_input.log_files.is_empty()
        {
            LogInspectEvidence::Unreachable {
                reason: InspectReason::SupportedLogSourceUnreachable,
            }
        } else {
            LogInspectEvidence::NotRequested
        };
    }

    let mut all_entries = Vec::new();
    for log_file in log_files {
        if let Ok(mut entries) = process_log_file(&log_file, parser, local_input.sample_size) {
            all_entries.append(&mut entries);
        }
    }

    if all_entries.is_empty() {
        LogInspectEvidence::Unreachable {
            reason: InspectReason::SupportedLogSourceUnreachable,
        }
    } else {
        LogInspectEvidence::Available(LogEvidence {
            entries: all_entries,
            source_kind,
        })
    }
}

pub fn format_inspect_text(report: &PgTriageReport<InspectReportPayload>) -> String {
    let inspect = &report.payload.inspect;
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
    for (name, check) in &inspect.database_inspect.checks {
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

    output.push_str("Agent Checks:\n");
    for (name, target) in [
        ("codex", &inspect.agent_inspect.codex),
        ("claude", &inspect.agent_inspect.claude),
        ("gemini", &inspect.agent_inspect.gemini),
    ] {
        output.push_str(&format!(
            "- {name}: {} ({})\n",
            check_status_label(target.status),
            target.install_location
        ));
    }

    if !inspect.recommended_next_commands.is_empty() {
        output.push_str("Recommended Next Commands:\n");
        for command in &inspect.recommended_next_commands {
            output.push_str(&format!("- {command}\n"));
        }
    }

    output
}

fn build_log_checks(log_evidence: LogInspectEvidence) -> BTreeMap<InspectCheckId, InspectCheck> {
    let mut checks = BTreeMap::new();
    let log_evidence = match log_evidence {
        LogInspectEvidence::NotRequested => {
            for name in [
                InspectCheckId::LogSourceReachable,
                InspectCheckId::StatementEvidence,
                InspectCheckId::DurationEvidence,
                InspectCheckId::CorrelationEvidence,
            ] {
                checks.insert(
                    name,
                    skipped_check(Some(InspectReason::LogSourceNotRequested), None),
                );
            }
            return checks;
        }
        LogInspectEvidence::Unreachable { reason } => {
            checks.insert(
                InspectCheckId::LogSourceReachable,
                failed_check(None, Some(reason)),
            );
            for name in [
                InspectCheckId::StatementEvidence,
                InspectCheckId::DurationEvidence,
                InspectCheckId::CorrelationEvidence,
            ] {
                checks.insert(
                    name,
                    skipped_check(Some(InspectReason::LogSourceUnreachable), None),
                );
            }
            return checks;
        }
        LogInspectEvidence::Available(log_evidence) => log_evidence,
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
        InspectCheckId::LogSourceReachable,
        passed_check(Some(json!(entries.len())), None),
    );
    checks.insert(
        InspectCheckId::StatementEvidence,
        if has_statement {
            passed_check(Some(json!(true)), None)
        } else {
            failed_check(
                Some(json!(false)),
                Some(InspectReason::StatementEvidenceMissing),
            )
        },
    );
    checks.insert(
        InspectCheckId::DurationEvidence,
        if has_duration {
            passed_check(Some(json!(true)), None)
        } else {
            failed_check(
                Some(json!(false)),
                Some(InspectReason::DurationEvidenceMissing),
            )
        },
    );
    checks.insert(
        InspectCheckId::CorrelationEvidence,
        if has_correlated_execution {
            passed_check(Some(json!(true)), None)
        } else {
            failed_check(
                Some(json!(false)),
                Some(InspectReason::CorrelationEvidenceMissing),
            )
        },
    );

    checks
}

fn build_database_checks(
    dsn: Option<String>,
    config: &AppConfig,
) -> BTreeMap<InspectCheckId, InspectCheck> {
    let mut checks = BTreeMap::new();
    let Some(dsn) = dsn else {
        for name in [
            InspectCheckId::LogDestination,
            InspectCheckId::LogLinePrefix,
            InspectCheckId::LogDuration,
            InspectCheckId::LogMinDurationStatement,
            InspectCheckId::LogTempFiles,
            InspectCheckId::TrackActivities,
            InspectCheckId::SharedPreloadLibraries,
            InspectCheckId::ComputeQueryId,
            InspectCheckId::PgStatStatementsExtension,
            InspectCheckId::PgReadAllStats,
            InspectCheckId::PgStatActivityProbe,
            InspectCheckId::PgStatStatementsProbe,
        ] {
            checks.insert(
                name,
                skipped_check(Some(InspectReason::DatabaseConnectionNotConfigured), None),
            );
        }
        return checks;
    };

    let mut client = match connect_postgres_client(&dsn, config.database.connect_timeout_ms) {
        Ok(client) => client,
        Err(reason) => {
            for name in [
                InspectCheckId::LogDestination,
                InspectCheckId::LogLinePrefix,
                InspectCheckId::LogDuration,
                InspectCheckId::LogMinDurationStatement,
                InspectCheckId::LogTempFiles,
                InspectCheckId::TrackActivities,
                InspectCheckId::SharedPreloadLibraries,
                InspectCheckId::ComputeQueryId,
                InspectCheckId::PgStatStatementsExtension,
                InspectCheckId::PgReadAllStats,
                InspectCheckId::PgStatActivityProbe,
                InspectCheckId::PgStatStatementsProbe,
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
        InspectCheckId::LogDestination,
        evaluate_log_destination(show_setting(&mut client, "log_destination").as_deref()),
    );
    checks.insert(
        InspectCheckId::LogLinePrefix,
        evaluate_log_line_prefix(show_setting(&mut client, "log_line_prefix").as_deref()),
    );
    checks.insert(
        InspectCheckId::LogDuration,
        evaluate_on_off_setting(
            show_setting(&mut client, "log_duration").as_deref(),
            InspectReason::LogDurationDisabled,
        ),
    );
    checks.insert(
        InspectCheckId::LogMinDurationStatement,
        evaluate_non_negative_setting(
            show_setting(&mut client, "log_min_duration_statement").as_deref(),
            InspectReason::LogMinDurationStatementDisabled,
        ),
    );
    checks.insert(
        InspectCheckId::LogTempFiles,
        evaluate_non_negative_setting(
            show_setting(&mut client, "log_temp_files").as_deref(),
            InspectReason::LogTempFilesDisabled,
        ),
    );
    checks.insert(
        InspectCheckId::TrackActivities,
        evaluate_on_off_setting(
            show_setting(&mut client, "track_activities").as_deref(),
            InspectReason::TrackActivitiesDisabled,
        ),
    );
    checks.insert(
        InspectCheckId::SharedPreloadLibraries,
        evaluate_list_contains(
            show_setting(&mut client, "shared_preload_libraries").as_deref(),
            "pg_stat_statements",
            InspectReason::PgStatStatementsNotPreloaded,
        ),
    );
    checks.insert(
        InspectCheckId::ComputeQueryId,
        evaluate_allowed_values(
            show_setting(&mut client, "compute_query_id").as_deref(),
            &["auto", "on"],
            InspectReason::ComputeQueryIdDisabled,
        ),
    );
    checks.insert(
        InspectCheckId::PgStatStatementsExtension,
        evaluate_exists_query(
            &mut client,
            "SELECT EXISTS (SELECT 1 FROM pg_extension WHERE extname = 'pg_stat_statements')",
            InspectReason::PgStatStatementsExtensionMissing,
        ),
    );
    checks.insert(
        InspectCheckId::PgReadAllStats,
        evaluate_exists_query(
            &mut client,
            "SELECT pg_has_role(current_user, 'pg_read_all_stats', 'member') OR (SELECT rolsuper FROM pg_roles WHERE rolname = current_user)",
            InspectReason::PgReadAllStatsUnavailable,
        ),
    );
    checks.insert(
        InspectCheckId::PgStatActivityProbe,
        evaluate_probe_query(
            &mut client,
            "SELECT pid, datname, usename, application_name, state, wait_event_type, wait_event, query_id, query FROM pg_stat_activity LIMIT 1",
        ),
    );
    checks.insert(
        InspectCheckId::PgStatStatementsProbe,
        evaluate_probe_query(
            &mut client,
            "SELECT queryid, query, calls, total_exec_time, mean_exec_time FROM pg_stat_statements ORDER BY total_exec_time DESC LIMIT 1",
        ),
    );

    checks
}

fn detect_agent_inspect(config: &AppConfig) -> AgentInspect {
    AgentInspect {
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

fn detect_target(configured: Option<PathBuf>, default: Option<PathBuf>) -> AgentTargetInspect {
    let path = configured.or(default);
    let installed = path.as_ref().is_some_and(|path| path.exists());
    let install_location = path
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "unconfigured".to_string());

    AgentTargetInspect {
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
    base: &mut BTreeMap<InspectCheckId, InspectCheck>,
    additional: BTreeMap<InspectCheckId, InspectCheck>,
) {
    for (name, check) in additional {
        base.insert(name, check);
    }
}

fn determine_mode(checks: &BTreeMap<InspectCheckId, InspectCheck>) -> OperatingMode {
    let log_backed_ready = passed(checks, InspectCheckId::LogSourceReachable)
        && passed(checks, InspectCheckId::StatementEvidence)
        && passed(checks, InspectCheckId::DurationEvidence)
        && passed(checks, InspectCheckId::CorrelationEvidence);
    if log_backed_ready {
        return OperatingMode::LogBacked;
    }

    let live_only_ready = passed(checks, InspectCheckId::TrackActivities)
        && passed(checks, InspectCheckId::SharedPreloadLibraries)
        && passed(checks, InspectCheckId::ComputeQueryId)
        && passed(checks, InspectCheckId::PgStatStatementsExtension)
        && passed(checks, InspectCheckId::PgReadAllStats)
        && passed(checks, InspectCheckId::PgStatActivityProbe)
        && passed(checks, InspectCheckId::PgStatStatementsProbe);

    if live_only_ready {
        OperatingMode::LiveOnly
    } else {
        OperatingMode::Unready
    }
}

fn collect_failed_checks(checks: &BTreeMap<InspectCheckId, InspectCheck>) -> Vec<InspectReason> {
    checks
        .values()
        .filter(|check| matches!(check.status, CheckStatus::Failed))
        .filter_map(|check| check.reason)
        .collect()
}

fn build_limitations(
    mode: OperatingMode,
    checks: &BTreeMap<InspectCheckId, InspectCheck>,
) -> Vec<String> {
    match mode {
        OperatingMode::LogBacked => {
            let mut limitations = Vec::new();
            if checks
                .get(&InspectCheckId::PgStatActivityProbe)
                .is_some_and(|check| matches!(check.status, CheckStatus::Skipped))
            {
                limitations.push(
                    InspectLimitation::LiveDatabaseChecksUnavailable
                        .as_str()
                        .to_string(),
                );
            }
            limitations
        }
        OperatingMode::LiveOnly => vec![
            InspectLimitation::HistoricalLogTriageUnavailable
                .as_str()
                .to_string(),
            InspectLimitation::EventLevelEvidenceUnavailable
                .as_str()
                .to_string(),
        ],
        OperatingMode::Unready => {
            let mut limitations = vec![InspectLimitation::SupportedEvidenceUnavailable
                .as_str()
                .to_string()];
            if checks
                .get(&InspectCheckId::PgStatActivityProbe)
                .is_some_and(|check| matches!(check.status, CheckStatus::Skipped))
            {
                limitations.push(
                    InspectLimitation::DatabaseConnectionNotConfigured
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
            "pg-logstats inspect --output-format json --dsn <postgres-url>".to_string(),
            "pg-logstats inspect --output-format json <log-file>".to_string(),
        ],
    }
}

fn passed(checks: &BTreeMap<InspectCheckId, InspectCheck>, name: InspectCheckId) -> bool {
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

fn evaluate_log_destination(value: Option<&str>) -> InspectCheck {
    let Some(value) = value else {
        return failed_check(None, Some(InspectReason::LogDestinationUnavailable));
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
            Some(InspectReason::UnsupportedLogFormat),
        )
    } else {
        failed_check(
            Some(json!(value)),
            Some(InspectReason::UnsupportedLogDestination),
        )
    }
}

fn evaluate_log_line_prefix(value: Option<&str>) -> InspectCheck {
    let Some(value) = value else {
        return failed_check(None, Some(InspectReason::LogLinePrefixUnavailable));
    };

    if value.contains("%p") {
        passed_check(Some(json!(value)), None)
    } else {
        failed_check(
            Some(json!(value)),
            Some(InspectReason::LogLinePrefixMissingProcessId),
        )
    }
}

fn evaluate_on_off_setting(value: Option<&str>, failure_reason: InspectReason) -> InspectCheck {
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
    failure_reason: InspectReason,
) -> InspectCheck {
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
    failure_reason: InspectReason,
) -> InspectCheck {
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
    failure_reason: InspectReason,
) -> InspectCheck {
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
    failure_reason: InspectReason,
) -> InspectCheck {
    match client.query_one(sql, &[]) {
        Ok(row) => match row.try_get::<_, bool>(0) {
            Ok(true) => passed_check(Some(json!(true)), None),
            Ok(false) => failed_check(Some(json!(false)), Some(failure_reason)),
            Err(_) => failed_check(None, Some(InspectReason::ProbeFailed)),
        },
        Err(_) => failed_check(None, Some(InspectReason::ProbeFailed)),
    }
}

fn evaluate_probe_query(client: &mut Client, sql: &str) -> InspectCheck {
    match client.query(sql, &[]) {
        Ok(_) => passed_check(None, None),
        Err(_) => failed_check(None, Some(InspectReason::ProbeFailed)),
    }
}

fn split_csv_setting(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|item| item.trim().to_string())
        .collect()
}

fn passed_check(value: Option<Value>, reason: Option<InspectReason>) -> InspectCheck {
    InspectCheck {
        status: CheckStatus::Passed,
        value,
        reason,
    }
}

fn failed_check(value: Option<Value>, reason: Option<InspectReason>) -> InspectCheck {
    InspectCheck {
        status: CheckStatus::Failed,
        value,
        reason,
    }
}

fn skipped_check(reason: Option<InspectReason>, value: Option<Value>) -> InspectCheck {
    InspectCheck {
        status: CheckStatus::Skipped,
        value,
        reason,
    }
}

fn parse_connection_reason(reason: &str) -> InspectReason {
    if reason.starts_with("database_connection_invalid") {
        InspectReason::DatabaseConnectionInvalid
    } else {
        InspectReason::DatabaseConnectionFailed
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

fn check_id_label(id: InspectCheckId) -> &'static str {
    match id {
        InspectCheckId::LogSourceReachable => "log_source_reachable",
        InspectCheckId::StatementEvidence => "statement_evidence",
        InspectCheckId::DurationEvidence => "duration_evidence",
        InspectCheckId::CorrelationEvidence => "correlation_evidence",
        InspectCheckId::LogDestination => "log_destination",
        InspectCheckId::LogLinePrefix => "log_line_prefix",
        InspectCheckId::LogDuration => "log_duration",
        InspectCheckId::LogMinDurationStatement => "log_min_duration_statement",
        InspectCheckId::LogTempFiles => "log_temp_files",
        InspectCheckId::TrackActivities => "track_activities",
        InspectCheckId::SharedPreloadLibraries => "shared_preload_libraries",
        InspectCheckId::ComputeQueryId => "compute_query_id",
        InspectCheckId::PgStatStatementsExtension => "pg_stat_statements_extension",
        InspectCheckId::PgReadAllStats => "pg_read_all_stats",
        InspectCheckId::PgStatActivityProbe => "pg_stat_activity_probe",
        InspectCheckId::PgStatStatementsProbe => "pg_stat_statements_probe",
    }
}

fn reason_label(reason: InspectReason) -> &'static str {
    match reason {
        InspectReason::LogSourceNotRequested => "log_source_not_requested",
        InspectReason::LogSourceUnreachable => "log_source_unreachable",
        InspectReason::SupportedLogSourceUnreachable => "supported_log_source_unreachable",
        InspectReason::DatabaseConnectionNotConfigured => "database_connection_not_configured",
        InspectReason::DatabaseConnectionInvalid => "database_connection_invalid",
        InspectReason::DatabaseConnectionFailed => "database_connection_failed",
        InspectReason::LogDestinationUnavailable => "log_destination_unavailable",
        InspectReason::UnsupportedLogFormat => "unsupported_log_format",
        InspectReason::UnsupportedLogDestination => "unsupported_log_destination",
        InspectReason::LogLinePrefixUnavailable => "log_line_prefix_unavailable",
        InspectReason::LogLinePrefixMissingProcessId => "log_line_prefix_missing_process_id",
        InspectReason::LogDurationDisabled => "log_duration_disabled",
        InspectReason::LogMinDurationStatementDisabled => "log_min_duration_statement_disabled",
        InspectReason::LogTempFilesDisabled => "log_temp_files_disabled",
        InspectReason::TrackActivitiesDisabled => "track_activities_disabled",
        InspectReason::PgStatStatementsNotPreloaded => "pg_stat_statements_not_preloaded",
        InspectReason::ComputeQueryIdDisabled => "compute_query_id_disabled",
        InspectReason::PgStatStatementsExtensionMissing => "pg_stat_statements_extension_missing",
        InspectReason::PgReadAllStatsUnavailable => "pg_read_all_stats_unavailable",
        InspectReason::StatementEvidenceMissing => "statement_evidence_missing",
        InspectReason::DurationEvidenceMissing => "duration_evidence_missing",
        InspectReason::CorrelationEvidenceMissing => "correlation_evidence_missing",
        InspectReason::ProbeFailed => "probe_failed",
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
    fn inspect_uses_log_evidence_without_database_connection() {
        let report = build_inspect_report(
            &AppConfig::default(),
            None,
            LogInspectEvidence::Available(sample_log_evidence()),
        );

        assert_eq!(report.operating_mode, OperatingMode::LogBacked);
        assert_eq!(
            report.payload.inspect.database_inspect.checks[&InspectCheckId::PgStatActivityProbe]
                .status,
            CheckStatus::Skipped
        );
    }

    #[test]
    fn inspect_without_any_evidence_is_unready() {
        let report = build_inspect_report(
            &AppConfig::default(),
            None,
            LogInspectEvidence::NotRequested,
        );

        assert_eq!(report.operating_mode, OperatingMode::Unready);
        assert!(report.payload.inspect.failed_checks.is_empty());
        assert!(report.limitations.contains(
            &InspectLimitation::DatabaseConnectionNotConfigured
                .as_str()
                .to_string()
        ));
    }

    #[test]
    fn unreachable_log_source_records_failed_check() {
        let report = build_inspect_report(
            &AppConfig::default(),
            None,
            LogInspectEvidence::Unreachable {
                reason: InspectReason::SupportedLogSourceUnreachable,
            },
        );

        assert_eq!(
            report.payload.inspect.database_inspect.checks[&InspectCheckId::LogSourceReachable]
                .status,
            CheckStatus::Failed
        );
    }
}
