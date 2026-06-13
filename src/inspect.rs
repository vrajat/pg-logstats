//! Environment inspection logic for detecting PostgreSQL operating capability modes,
//! verifying log evidence formats, database configurations, and active agent installations.

use crate::config::{AgentInstallTargetConfig, AppConfig};
use crate::database::connect_postgres_client;
use crate::guidance::{GuidancePayload, RuleDefinition, RuleId};
use crate::input::{
    discover_log_files, process_cloudwatch_input, process_log_file, CloudWatchInput, LocalLogInput,
};
use crate::triage::{
    ActionKind, CheckStatus, NextActionPriority, OperatingMode, PgTriageReport,
    PG_TRIAGE_SCHEMA_VERSION,
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

/// Default number of log lines to parse during inspection if no sample size is explicitly requested.
const DEFAULT_INSPECT_SAMPLE_SIZE: usize = 1000;

/// The check IDs that verify the availability and format of log files.
const LOG_CHECK_IDS: &[InspectCheckId] = &[
    InspectCheckId::LogSourceReachable,
    InspectCheckId::StatementEvidence,
    InspectCheckId::DurationEvidence,
    InspectCheckId::CorrelationEvidence,
];

/// The check IDs that query and verify PostgreSQL configuration settings and views.
const DATABASE_CHECK_IDS: &[InspectCheckId] = &[
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

/// The list of checks required to determine operating capabilities.
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

/// Top-level report payload for the environment inspection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectReportPayload {
    /// Database settings and connection check results.
    pub database_inspect: DatabaseInspect,
    /// AI agent integration install check results.
    pub agent_inspect: AgentInspect,
    /// The list of all checks run during this inspection.
    pub required_checks: Vec<InspectCheckId>,
    /// Reasons for check failures, if any.
    pub failed_checks: Vec<InspectReason>,
}

/// Verification results for database configuration settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseInspect {
    /// The operating mode supported by the current environment configuration.
    pub mode_candidate: OperatingMode,
    /// Map of check IDs to their status, verified value, and optional failure reason.
    pub checks: BTreeMap<InspectCheckId, InspectCheck>,
}

/// Installation check results for the supported AI agent integrations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInspect {
    /// The active agent harness configured, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_harness: Option<String>,
    /// Status of the Codex `AGENTS.md` system prompt wrapper.
    pub codex: AgentTargetInspect,
    /// Status of the Claude skill file.
    pub claude: AgentTargetInspect,
    /// Status of the Gemini command configuration.
    pub gemini: AgentTargetInspect,
}

/// The status and location of a specific agent integration target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTargetInspect {
    /// Verification status of the integration.
    pub status: CheckStatus,
    /// Whether the integration artifact is installed.
    pub installed: bool,
    /// The path or location where the integration was detected or configured.
    pub install_location: String,
}

/// The result of an individual environment or database check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectCheck {
    /// Whether the check passed, failed, or was skipped.
    pub status: CheckStatus,
    /// The setting value retrieved from PostgreSQL or log files, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    /// The reason for check failure or skipping, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<InspectReason>,
}

/// Unique identifiers for individual checks run during environment inspection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InspectCheckId {
    /// Verifies that log files can be discovered and read.
    LogSourceReachable,
    /// Verifies that at least one query statement is present in logs.
    StatementEvidence,
    /// Verifies that at least one duration log is present.
    DurationEvidence,
    /// Verifies that at least one statement can be correlated with a duration.
    CorrelationEvidence,
    /// Checks PostgreSQL `log_destination` GUC.
    LogDestination,
    /// Checks PostgreSQL `log_line_prefix` GUC.
    LogLinePrefix,
    /// Checks PostgreSQL `log_duration` GUC.
    LogDuration,
    /// Checks PostgreSQL `log_min_duration_statement` GUC.
    LogMinDurationStatement,
    /// Checks PostgreSQL `log_temp_files` GUC.
    LogTempFiles,
    /// Checks PostgreSQL `track_activities` GUC.
    TrackActivities,
    /// Verifies that `pg_stat_statements` is preloaded in `shared_preload_libraries`.
    SharedPreloadLibraries,
    /// Checks PostgreSQL `compute_query_id` GUC.
    ComputeQueryId,
    /// Verifies that the `pg_stat_statements` extension is installed.
    PgStatStatementsExtension,
    /// Verifies that the current user has the `pg_read_all_stats` role or superuser status.
    PgReadAllStats,
    /// Probes the `pg_stat_activity` system view to verify read permissions.
    PgStatActivityProbe,
    /// Probes the `pg_stat_statements` view to verify query stats can be read.
    PgStatStatementsProbe,
}

/// Diagnostic failure or skip reasons for environment verification checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InspectReason {
    /// Log checks were skipped because log paths were not supplied.
    LogSourceNotRequested,
    /// Log check skipped because log files are unreachable.
    LogSourceUnreachable,
    /// Log check failed because log files or directories cannot be read.
    SupportedLogSourceUnreachable,
    /// Database check skipped because no DSN was configured.
    DatabaseConnectionNotConfigured,
    /// Database check failed because the connection string is malformed.
    DatabaseConnectionInvalid,
    /// Database check failed because connection to the server timed out or failed.
    DatabaseConnectionFailed,
    /// `log_destination` could not be queried.
    LogDestinationUnavailable,
    /// Log destination is set to unsupported formats (e.g. csvlog or jsonlog).
    UnsupportedLogFormat,
    /// Log destination is not set to `stderr`.
    UnsupportedLogDestination,
    /// `log_line_prefix` could not be queried.
    LogLinePrefixUnavailable,
    /// `log_line_prefix` does not include process ID (`%p`).
    LogLinePrefixMissingProcessId,
    /// `log_duration` is disabled (`off`).
    LogDurationDisabled,
    /// `log_min_duration_statement` is disabled (`-1`).
    LogMinDurationStatementDisabled,
    /// `log_temp_files` is disabled (`-1`).
    LogTempFilesDisabled,
    /// `track_activities` is disabled (`off`).
    TrackActivitiesDisabled,
    /// `pg_stat_statements` is missing from `shared_preload_libraries`.
    PgStatStatementsNotPreloaded,
    /// `compute_query_id` is disabled (`off` or `regress`).
    ComputeQueryIdDisabled,
    /// The `pg_stat_statements` extension is missing from the database.
    PgStatStatementsExtensionMissing,
    /// Current user does not have `pg_read_all_stats` permission.
    PgReadAllStatsUnavailable,
    /// No statement log evidence found in parsed files.
    StatementEvidenceMissing,
    /// No duration log evidence found in parsed files.
    DurationEvidenceMissing,
    /// Failed to correlate statement and duration lines in parsed logs.
    CorrelationEvidenceMissing,
    /// Verification query against system view failed.
    ProbeFailed,
}

/// Verification limitations representing degraded diagnostic capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectLimitation {
    /// Live database diagnostic queries cannot be run.
    LiveDatabaseChecksUnavailable,
    /// Historical query parsing and baseline diffing cannot be run.
    HistoricalLogTriageUnavailable,
    /// Individual query event parsing and execution flow tracking is unavailable.
    EventLevelEvidenceUnavailable,
    /// Neither database connection nor parsed logs were successfully inspected.
    SupportedEvidenceUnavailable,
    /// Database checks were skipped because connection DSN is missing.
    DatabaseConnectionNotConfigured,
}

impl InspectLimitation {
    /// Returns the string key representation of the limitation.
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

/// Container for the list of parsed log entries during inspection.
#[derive(Debug, Clone)]
pub struct LogEvidence {
    /// The log entries successfully read and parsed.
    pub entries: Vec<LogEntry>,
    /// The detected log line format source kind.
    pub source_kind: EventSourceKind,
}

/// Represents the status of collected log evidence.
#[derive(Debug, Clone)]
pub enum LogInspectEvidence {
    /// No logs were requested to be parsed.
    NotRequested,
    /// Logs were requested but could not be reached.
    Unreachable { reason: InspectReason },
    /// Logs were successfully read and parsed.
    Available(LogEvidence),
}

// =========================================================================
// Public Functions
// =========================================================================

/// Runs the inspect command workflow: collects log evidence, queries the database if configured,
/// determines operating mode, persists the report in the workspace, and returns it.
#[allow(clippy::too_many_arguments)]
pub fn inspect(
    config: &AppConfig,
    dsn: Option<&str>,
    local_log_input: &LocalLogInput,
    cloudwatch_input: Option<&CloudWatchInput>,
    parser: &TextLogParser,
    source_kind: EventSourceKind,
    workspace: Option<&Path>,
) -> Result<PgTriageReport<InspectReportPayload>, crate::PgLogstatsError> {
    let log_evidence =
        collect_log_inspect_evidence(local_log_input, cloudwatch_input, parser, source_kind);
    let report = build_inspect_report(
        config,
        crate::database::resolve_database_dsn(dsn, config).as_deref(),
        log_evidence,
    );

    persist_inspect_report(&report, workspace)?;

    Ok(report)
}

/// Formats the inspect triage report into human-readable console text.
pub fn format_inspect_text(report: &PgTriageReport<InspectReportPayload>) -> String {
    let inspect = &report.payload;
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

    output
}

/// Returns the guidance rules definitions associated with the environment inspection workflow.
pub fn inspect_rules() -> Vec<RuleDefinition> {
    vec![
        RuleDefinition {
            rule_id: RuleId::InspectTopQueryFamilies,
            emitted_action_id: RuleId::InspectTopQueryFamilies,
            kind: ActionKind::TopQueryFamilies,
            target_workflow: ActionKind::Inspect,
            target_finding_kind: None,
            destination_workflow: Some(ActionKind::TopQueryFamilies),
            required_identifiers: vec![],
            label: "Rank query families from the available log window".to_string(),
            reason: "Log-backed mode is available. Use this when the incident appears query-latency related.".to_string(),
            priority: NextActionPriority::Recommended,
            risk: None,
            action_class: None,
            command_template: Some(vec![
                "pg-logstats".to_string(),
                "top".to_string(),
                "query-families".to_string(),
                "--output-format".to_string(),
                "json".to_string(),
            ]),
            sql_template: None,
            required_operating_mode: Some(OperatingMode::LogBackedOnly),
            produces: vec!["workflow:top_query_families".to_string()],
            attribution: "PostgreSQL logging configuration and evidence prerequisites".to_string(),
        },
        RuleDefinition {
            rule_id: RuleId::InspectRunningQueries,
            emitted_action_id: RuleId::InspectRunningQueries,
            kind: ActionKind::RunningQueries,
            target_workflow: ActionKind::Inspect,
            target_finding_kind: None,
            destination_workflow: Some(ActionKind::RunningQueries),
            required_identifiers: vec![],
            label: "Check active and waiting queries".to_string(),
            reason: "Live activity checking is available. Use this to inspect ongoing database load.".to_string(),
            priority: NextActionPriority::Recommended,
            risk: None,
            action_class: None,
            command_template: Some(vec![
                "pg-logstats".to_string(),
                "running-queries".to_string(),
                "--output-format".to_string(),
                "json".to_string(),
            ]),
            sql_template: None,
            required_operating_mode: Some(OperatingMode::LiveOnly),
            produces: vec!["workflow:running_queries".to_string()],
            attribution: "pg_stat_activity monitoring requirements".to_string(),
        },
        RuleDefinition {
            rule_id: RuleId::InspectAgentInstall,
            emitted_action_id: RuleId::InspectAgentInstall,
            kind: ActionKind::AgentInstall,
            target_workflow: ActionKind::Inspect,
            target_finding_kind: None,
            destination_workflow: Some(ActionKind::AgentInstall),
            required_identifiers: vec![],
            label: "Install pg-logstats agent integrations".to_string(),
            reason: "One or more agent integrations (Claude skill, Gemini command, or Codex AGENTS.md) are missing or failed verification.".to_string(),
            priority: NextActionPriority::Recommended,
            risk: None,
            action_class: None,
            command_template: Some(vec![
                "pg-logstats".to_string(),
                "agent-install".to_string(),
            ]),
            sql_template: None,
            required_operating_mode: None,
            produces: vec!["workflow:agent_install".to_string()],
            attribution: "Agent installation setup".to_string(),
        },
    ]
}

impl GuidancePayload for InspectReportPayload {
    fn evaluate_rules(
        &self,
        operating_mode: OperatingMode,
        verdict: Option<crate::triage::Verdict>,
        config: &AppConfig,
    ) -> Vec<crate::triage::NextAction> {
        let rules = inspect_rules();
        let mut actions = Vec::new();

        for rule in rules {
            let (mut status, mut reason) =
                crate::guidance::evaluate_rule_constraints(&rule, operating_mode, verdict, config);

            if rule.rule_id == RuleId::InspectAgentInstall {
                let mut missing_agents = Vec::new();
                if let Some(harness) = &self.agent_inspect.active_harness {
                    let agent_val = match harness.as_str() {
                        "codex" => &self.agent_inspect.codex,
                        "claude" => &self.agent_inspect.claude,
                        "gemini" => &self.agent_inspect.gemini,
                        _ => unreachable!(),
                    };
                    if agent_val.status != CheckStatus::Passed {
                        missing_agents.push(harness.clone());
                    }
                }
                if missing_agents.is_empty() {
                    status = crate::triage::NextActionStatus::OmittedNotEnoughContext;
                    reason = "All agent integrations are fully installed.".to_string();
                } else {
                    reason = format!(
                        "One or more agent integrations ({}) are missing or failed verification.",
                        missing_agents.join(", ")
                    );
                }
            }

            if status == crate::triage::NextActionStatus::OmittedNotEnoughContext
                && !config.guidance.show_omitted
            {
                continue;
            }

            let next_act = crate::guidance::build_next_action(
                &rule,
                status,
                reason,
                None,
                rule.command_template
                    .clone()
                    .map(|argv| crate::triage::NextActionCommand { argv }),
                None,
            );
            actions.push(next_act);
        }

        // If agent install is allowed, it should be the only recommended action
        if actions.iter().any(|a| {
            a.action_id == "inspect.agent_install"
                && a.status == crate::triage::NextActionStatus::Allowed
        }) {
            actions.retain(|a| a.action_id == "inspect.agent_install");
        }

        actions
    }
}

// =========================================================================
// Report Building Helpers
// =========================================================================

/// Builds a structured inspect report from the database and log check outputs.
fn build_inspect_report(
    config: &AppConfig,
    resolved_dsn: Option<&str>,
    log_evidence: LogInspectEvidence,
) -> PgTriageReport<InspectReportPayload> {
    let mut checks = BTreeMap::new();
    build_log_checks(log_evidence, &mut checks);
    build_database_checks(resolved_dsn.map(str::to_string), config, &mut checks);

    let mode_candidate = determine_mode(&checks);
    let failed_checks = collect_failed_checks(&checks);
    let limitations = build_limitations(mode_candidate, &checks);
    let agent_inspect = inspect_agent_integrations(config);

    let mut report = PgTriageReport {
        schema_version: PG_TRIAGE_SCHEMA_VERSION,
        workflow: ActionKind::Inspect,
        operating_mode: mode_candidate,
        limitations,
        verdict: None,
        verdict_reasons: Vec::new(),
        allowed_actions: None,
        blocked_actions: None,
        analysis_window: None,
        source_summary: None,
        next_actions: Vec::new(),
        report_id: None,
        parent_report_id: None,
        selected_action_id: None,
        created_at: None,
        payload: InspectReportPayload {
            database_inspect: DatabaseInspect {
                mode_candidate,
                checks,
            },
            agent_inspect,
            required_checks: REQUIRED_CHECKS.to_vec(),
            failed_checks,
        },
    };

    crate::guidance::populate_next_actions(&mut report, config);
    report
}

/// Resolves the candidate operating mode based on passing verification checks.
fn determine_mode(checks: &BTreeMap<InspectCheckId, InspectCheck>) -> OperatingMode {
    let log_backed_ready = passed(checks, InspectCheckId::LogSourceReachable)
        && passed(checks, InspectCheckId::StatementEvidence)
        && passed(checks, InspectCheckId::DurationEvidence)
        && passed(checks, InspectCheckId::CorrelationEvidence);

    let live_ready = passed(checks, InspectCheckId::TrackActivities)
        && passed(checks, InspectCheckId::SharedPreloadLibraries)
        && passed(checks, InspectCheckId::ComputeQueryId)
        && passed(checks, InspectCheckId::PgStatStatementsExtension)
        && passed(checks, InspectCheckId::PgReadAllStats)
        && passed(checks, InspectCheckId::PgStatActivityProbe)
        && passed(checks, InspectCheckId::PgStatStatementsProbe);

    match (log_backed_ready, live_ready) {
        (true, true) => OperatingMode::LogBackedAndLive,
        (true, false) => OperatingMode::LogBackedOnly,
        (false, true) => OperatingMode::LiveOnly,
        (false, false) => OperatingMode::Unready,
    }
}

/// Accumulates the failure reasons from all verification checks.
fn collect_failed_checks(checks: &BTreeMap<InspectCheckId, InspectCheck>) -> Vec<InspectReason> {
    checks
        .values()
        .filter(|check| matches!(check.status, CheckStatus::Failed))
        .filter_map(|check| check.reason)
        .collect()
}

/// Populates structural triage capability limitations based on the operating mode.
fn build_limitations(
    mode: OperatingMode,
    checks: &BTreeMap<InspectCheckId, InspectCheck>,
) -> Vec<String> {
    match mode {
        OperatingMode::LogBackedAndLive => Vec::new(),
        OperatingMode::LogBackedOnly => vec![InspectLimitation::LiveDatabaseChecksUnavailable
            .as_str()
            .to_string()],
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

/// Persists the inspect report to the workspace (`inspect.json`).
fn persist_inspect_report(
    report: &PgTriageReport<InspectReportPayload>,
    workspace: Option<&Path>,
) -> Result<(), crate::PgLogstatsError> {
    let workspace = crate::config::resolve_workspace_path(workspace)?;
    let path = crate::config::workspace_inspect_report_path(&workspace);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let output =
        serde_json::to_string_pretty(report).map_err(crate::PgLogstatsError::Serialization)?;
    std::fs::write(path, output)?;
    Ok(())
}

// =========================================================================
// Log Verification & Checks Helpers
// =========================================================================

/// Scans local files or CloudWatch logs, limits lines, and parses them for log evidence.
fn collect_log_inspect_evidence(
    local_input: &LocalLogInput,
    cloudwatch_input: Option<&CloudWatchInput>,
    parser: &TextLogParser,
    source_kind: EventSourceKind,
) -> LogInspectEvidence {
    if let Some(cloudwatch_input) = cloudwatch_input {
        let mut cw = cloudwatch_input.clone();
        if cw.sample_size.is_none() {
            cw.sample_size = Some(DEFAULT_INSPECT_SAMPLE_SIZE);
        }
        return match process_cloudwatch_input(&cw, parser) {
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
    let sample_size = local_input
        .sample_size
        .or(Some(DEFAULT_INSPECT_SAMPLE_SIZE));
    for log_file in log_files {
        if let Ok(mut entries) = process_log_file(&log_file, parser, sample_size) {
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

/// Evaluates statement, duration, and correlation checks against the collected log evidence.
fn build_log_checks(
    log_evidence: LogInspectEvidence,
    checks: &mut BTreeMap<InspectCheckId, InspectCheck>,
) {
    let log_evidence = match log_evidence {
        LogInspectEvidence::NotRequested => {
            for &name in LOG_CHECK_IDS {
                checks.insert(
                    name,
                    skipped_check(Some(InspectReason::LogSourceNotRequested), None),
                );
            }
            return;
        }
        LogInspectEvidence::Unreachable { reason } => {
            checks.insert(
                InspectCheckId::LogSourceReachable,
                failed_check(None, Some(reason)),
            );
            for &name in LOG_CHECK_IDS {
                if name != InspectCheckId::LogSourceReachable {
                    checks.insert(
                        name,
                        skipped_check(Some(InspectReason::LogSourceUnreachable), None),
                    );
                }
            }
            return;
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
}

// =========================================================================
// Database Verification Helpers
// =========================================================================

/// Connects to PostgreSQL, checks GUC settings, extension availability, and queries probes.
fn build_database_checks(
    dsn: Option<String>,
    config: &AppConfig,
    checks: &mut BTreeMap<InspectCheckId, InspectCheck>,
) {
    let Some(dsn) = dsn else {
        for &name in DATABASE_CHECK_IDS {
            checks.insert(
                name,
                skipped_check(Some(InspectReason::DatabaseConnectionNotConfigured), None),
            );
        }
        return;
    };

    let mut client = match connect_postgres_client(&dsn, config.database.connect_timeout_ms) {
        Ok(client) => client,
        Err(reason) => {
            for &name in DATABASE_CHECK_IDS {
                checks.insert(
                    name,
                    failed_check(None, Some(parse_connection_reason(&reason))),
                );
            }
            return;
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
}

/// Executes a database GUC setting check.
fn show_setting(client: &mut Client, setting: &str) -> Option<String> {
    let sql = format!("SHOW {setting}");
    client
        .query_one(&sql, &[])
        .ok()
        .and_then(|row| row.try_get::<_, String>(0).ok())
}

/// Evaluates if PostgreSQL logs are redirected to stderr format.
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

/// Verifies if the log line prefix contains the mandatory process ID (%p).
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

/// Evaluates Boolean settings (e.g., track_activities, log_duration).
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

/// Evaluates non-negative integer settings (e.g., log_temp_files, log_min_duration_statement).
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

/// Checks if a setting value matches one of the expected/allowed configurations.
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

/// Verifies if a CSV list setting contains a specific value (e.g. pg_stat_statements in shared_preload_libraries).
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

/// Runs a query that returns a Boolean indicating existence (e.g. check for extension).
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

/// Runs a query to probe view read access.
fn evaluate_probe_query(client: &mut Client, sql: &str) -> InspectCheck {
    match client.query(sql, &[]) {
        Ok(_) => passed_check(None, None),
        Err(_) => failed_check(None, Some(InspectReason::ProbeFailed)),
    }
}

/// Splits a CSV string into a trimmed vector of items.
fn split_csv_setting(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|item| item.trim().to_string())
        .collect()
}

/// Translates connection error messages to InspectReasons.
fn parse_connection_reason(reason: &str) -> InspectReason {
    if reason.starts_with("database_connection_invalid") {
        InspectReason::DatabaseConnectionInvalid
    } else {
        InspectReason::DatabaseConnectionFailed
    }
}

// =========================================================================
// Agent Integration Inspection Helpers
// =========================================================================

/// Inspects the installation status of all agent triage integrations.
fn inspect_agent_integrations(config: &AppConfig) -> AgentInspect {
    let harness = config.agent_install.active_harness.as_deref();
    AgentInspect {
        active_harness: config.agent_install.active_harness.clone(),
        codex: if harness == Some("codex") {
            detect_target(
                config.agent_install.codex.agents_md_path.clone(),
                default_home_path("AGENTS.md"),
            )
        } else {
            skipped_target()
        },
        claude: if harness == Some("claude") {
            detect_target(
                target_artifact_path(&config.agent_install.claude, |dir| {
                    dir.join("pg-logstats-triage").join("SKILL.md")
                }),
                default_home_path(".claude/skills/pg-logstats-triage/SKILL.md"),
            )
        } else {
            skipped_target()
        },
        gemini: if harness == Some("gemini") {
            detect_target(
                target_artifact_path(&config.agent_install.gemini, |dir| {
                    dir.join("pg-logstats-triage.toml")
                }),
                default_home_path(".gemini/commands/pg-logstats-triage.toml"),
            )
        } else {
            skipped_target()
        },
    }
}

/// Helper constructor to return a skipped agent target status.
fn skipped_target() -> AgentTargetInspect {
    AgentTargetInspect {
        status: CheckStatus::Skipped,
        installed: false,
        install_location: "skipped (inactive harness)".to_string(),
    }
}

/// Resolves the absolute path to an agent integration config/metadata file.
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

/// Checks if a file path exists and creates an integration inspect status block.
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

/// Resolves paths relative to the current user's home directory.
fn default_home_path(relative_path: &str) -> Option<PathBuf> {
    env::var_os("HOME").map(|home| PathBuf::from(home).join(relative_path))
}

// =========================================================================
// Status & Format Helpers
// =========================================================================

/// Evaluates if a specific check has passed.
fn passed(checks: &BTreeMap<InspectCheckId, InspectCheck>, name: InspectCheckId) -> bool {
    checks
        .get(&name)
        .is_some_and(|check| matches!(check.status, CheckStatus::Passed))
}

/// Helper constructor to return a passed check result.
fn passed_check(value: Option<Value>, reason: Option<InspectReason>) -> InspectCheck {
    InspectCheck {
        status: CheckStatus::Passed,
        value,
        reason,
    }
}

/// Helper constructor to return a failed check result.
fn failed_check(value: Option<Value>, reason: Option<InspectReason>) -> InspectCheck {
    InspectCheck {
        status: CheckStatus::Failed,
        value,
        reason,
    }
}

/// Helper constructor to return a skipped check result.
fn skipped_check(reason: Option<InspectReason>, value: Option<Value>) -> InspectCheck {
    InspectCheck {
        status: CheckStatus::Skipped,
        value,
        reason,
    }
}

/// String formatter for check status.
fn check_status_label(status: CheckStatus) -> &'static str {
    match status {
        CheckStatus::Passed => "passed",
        CheckStatus::Failed => "failed",
        CheckStatus::Skipped => "skipped",
    }
}

/// String formatter for operating modes.
fn operating_mode_label(mode: OperatingMode) -> &'static str {
    match mode {
        OperatingMode::LogBackedAndLive => "log_backed_and_live",
        OperatingMode::LogBackedOnly => "log_backed_only",
        OperatingMode::LiveOnly => "live_only",
        OperatingMode::Unready => "unready",
    }
}

/// String formatter for check IDs.
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

/// String formatter for failure/skip reasons.
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

// =========================================================================
// Unit Tests
// =========================================================================

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

        assert_eq!(report.operating_mode, OperatingMode::LogBackedOnly);
        assert_eq!(
            report.payload.database_inspect.checks[&InspectCheckId::PgStatActivityProbe].status,
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
        assert!(report.payload.failed_checks.is_empty());
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
            report.payload.database_inspect.checks[&InspectCheckId::LogSourceReachable].status,
            CheckStatus::Failed
        );
    }
}
