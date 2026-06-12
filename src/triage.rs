use crate::guidance::{GuidancePayload, RuleDefinition, RuleId, DEFAULT_RULE_LIMIT};
use crate::{EventSourceKind, Finding, FindingSet, LogEntry};
use serde::{Deserialize, Serialize};

/// The version of the schema used for triage reports.
pub const PG_TRIAGE_SCHEMA_VERSION: u32 = 1;

/// Represents the operating mode of the investigation tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatingMode {
    /// Both log files are available and live database access is configured.
    LogBackedAndLive,
    /// Log files are available and analyzed, but live database connection is unavailable.
    LogBackedOnly,
    /// Live database access only (no logs are available).
    LiveOnly,
    /// The connection or logs are not configured/reachable.
    Unready,
}

/// The overall triage verdict on the health of the database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// No issues found.
    Clear,
    /// High concurrency or query volume.
    Busy,
    /// Resources are fully exhausted (CPU, disk, connections).
    Saturated,
    /// State or diagnostics are insufficient to determine a verdict.
    Unknown,
}

/// The estimated risk level of running a specific action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLabel {
    /// No risk, completely safe to run (e.g., standard metadata check).
    Safe,
    /// Bounded risk (reads a limited amount of data/stats).
    Bounded,
    /// May consume significant resources or run slowly.
    Expensive,
    /// Requires manual human approval before execution.
    RequiresHumanApproval,
}

/// Classification of actions based on their access and mutation profiles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionClass {
    /// Reads from standard system catalog views.
    SystemCatalogReads,
    /// Reads active queries bounded to a fast execution path.
    BoundedActivityQueries,
    /// Reads from statistics views (like pg_stat_statements).
    StatsViewReads,
    /// Searches/looks up query text patterns in statistics views.
    TextPatternStatsSearch,
    /// Explains queries without executing them (EXPLAIN).
    ExplainWithoutAnalyze,
    /// Select queries that are not bounded by limits.
    LargeUnboundedSelects,
    /// Explains and runs queries to get query plans (EXPLAIN ANALYZE).
    ExplainAnalyze,
    /// Write operations or administrative actions.
    WriteOrAdminAction,
}

/// The validation status of a recommended next action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NextActionStatus {
    /// Allowed to be executed.
    Allowed,
    /// Blocked because the current operating mode does not support it.
    BlockedByMode,
    /// Blocked because the current database verdict restricts it.
    BlockedByVerdict,
    /// Blocked by user configuration/disabled rules.
    BlockedByConfig,
    /// Blocked by connection or safety policy.
    BlockedByPolicy,
    /// Not enough context (missing query IDs, database name, etc.).
    OmittedNotEnoughContext,
    /// The target finding type is not supported by the action.
    OmittedUnsupportedTarget,
}

/// Types of diagnostic workflows and recommended actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    /// Inspect environment operating mode.
    Inspect,
    /// Rank query families by total runtime.
    TopQueryFamilies,
    /// Compare query behavior between two log windows.
    SlowQueriesDiff,
    /// Monitor active database sessions.
    RunningQueries,
    /// Execute a diagnostic SQL statement.
    RunSql,
    /// Install pg-logstats agents on the database.
    AgentInstall,
    /// Collect logs from the database server.
    CollectLogs,
    /// Escalate the incident to a human operator.
    Escalate,
    /// Stop/terminate execution.
    Stop,
    /// Error analysis.
    Errors,
    /// Temporary files analysis.
    TempFiles,
}

/// Priority of a recommended next action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NextActionPriority {
    /// Strongly recommended to resolve the issue.
    Required,
    /// Recommended next step for diagnostics.
    Recommended,
    /// Optional investigation step.
    Optional,
}

/// Arguments to invoke a command.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NextActionCommand {
    /// Command line arguments (argv).
    pub argv: Vec<String>,
}

/// A recommended next diagnostic action.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NextAction {
    /// The unique identifier of the action.
    pub action_id: String,
    /// The kind/type of action.
    pub kind: ActionKind,
    /// Descriptive label shown to the user.
    pub label: String,
    /// Current execution/policy status.
    pub status: NextActionStatus,
    /// Recommended priority.
    pub priority: NextActionPriority,
    /// Whether the action requires human judgement before running.
    pub judgement_required: bool,
    /// The reasoning behind this recommendation.
    pub reason: String,

    /// The target identifier (like a query family ID) if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// The destination workflow of this action.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow: Option<ActionKind>,
    /// The CLI command template to run the action.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<NextActionCommand>,
    /// Pre-generated SQL statement preview for reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sql_preview: Option<String>,
    /// Query or command parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Vec<String>>,
    /// Risk associated with execution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk: Option<RiskLabel>,
    /// Action classification category.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_class: Option<ActionClass>,
    /// Identifiers required to populate/run this action.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_identifiers: Option<Vec<String>>,
    /// Prerequisites required before executing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires: Option<Vec<String>>,
    /// Artifacts or context this action produces.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub produces: Option<Vec<String>>,
}

/// The status of a check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    /// Check succeeded.
    Passed,
    /// Check failed.
    Failed,
    /// Check was skipped.
    Skipped,
}

/// Source summary category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceSummaryKind {
    /// Local PostgreSQL stderr log.
    LocalStderr,
    /// AWS RDS log stream.
    AwsRds,
    /// CSV formatted log.
    Csvlog,
    /// JSON formatted log.
    Jsonlog,
}

impl From<EventSourceKind> for SourceSummaryKind {
    fn from(value: EventSourceKind) -> Self {
        match value {
            EventSourceKind::Stderr => Self::LocalStderr,
            EventSourceKind::AwsRds => Self::AwsRds,
            EventSourceKind::Csvlog => Self::Csvlog,
            EventSourceKind::Jsonlog => Self::Jsonlog,
        }
    }
}

/// The analysis time window bounds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisWindow {
    /// Beginning timestamp (RFC 3339).
    pub since: String,
    /// Ending timestamp (RFC 3339).
    pub until: String,
}

/// Summary details of the processed log source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSummary {
    /// Kind of log source.
    pub kind: SourceSummaryKind,
    /// Total entries scanned.
    pub entries_scanned: usize,
}

/// Triage payload holding query findings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingsPayload {
    /// List of diagnostic findings.
    pub findings: Vec<Finding>,
}

/// The structured triage report JSON format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PgTriageReport<T> {
    /// Schema version number.
    pub schema_version: u32,
    /// Current investigation workflow/report kind.
    pub workflow: ActionKind,
    /// Supported operating mode.
    pub operating_mode: OperatingMode,
    /// List of system or configuration limitations.
    pub limitations: Vec<String>,
    /// Database health triage verdict.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verdict: Option<Verdict>,
    /// Supporting reasoning for the verdict.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub verdict_reasons: Vec<String>,
    /// Allowed diagnostic action classes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_actions: Option<Vec<ActionClass>>,
    /// Blocked diagnostic action classes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_actions: Option<Vec<ActionClass>>,
    /// Log analysis time window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analysis_window: Option<AnalysisWindow>,
    /// Scanned log source summary.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_summary: Option<SourceSummary>,
    /// List of suggested next actions.
    #[serde(default)]
    pub next_actions: Vec<NextAction>,
    /// Unique report identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub report_id: Option<String>,
    /// Active investigation session identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Reference report identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_report_id: Option<String>,
    /// Selected next action identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_action_id: Option<String>,
    /// Creation time (RFC 3339).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    /// Generic workflow-specific payload.
    pub payload: T,
}

/// Helper function to build a top query families report.
pub fn top_query_families_report(
    findings: FindingSet,
    entries: &[LogEntry],
    source_kind: EventSourceKind,
) -> PgTriageReport<FindingsPayload> {
    PgTriageReport {
        schema_version: PG_TRIAGE_SCHEMA_VERSION,
        workflow: ActionKind::TopQueryFamilies,
        operating_mode: OperatingMode::LogBackedOnly,
        limitations: Vec::new(),
        verdict: Some(Verdict::Unknown),
        verdict_reasons: vec!["live_state_verdict_not_evaluated".to_string()],
        allowed_actions: None,
        blocked_actions: None,
        analysis_window: analysis_window(entries),
        source_summary: Some(SourceSummary {
            kind: source_kind.into(),
            entries_scanned: entries.len(),
        }),
        next_actions: Vec::new(),
        report_id: None,
        session_id: None,
        parent_report_id: None,
        selected_action_id: None,
        created_at: None,
        payload: FindingsPayload {
            findings: findings.findings,
        },
    }
}

pub fn findings_rules() -> Vec<RuleDefinition> {
    vec![RuleDefinition {
        rule_id: RuleId::QueryFamilyPgStatStatementsLookup,
        emitted_action_id: RuleId::QueryFamilyPgStatStatementsLookup,
        kind: ActionKind::RunSql,
        target_workflow: ActionKind::TopQueryFamilies,
        target_finding_kind: Some(crate::findings::FindingKind::QueryFamily),
        destination_workflow: Some(ActionKind::RunSql),
        required_identifiers: vec!["normalized_sql".to_string()],
        label: "Lookup query stats in pg_stat_statements".to_string(),
        reason: "Query normalized text is available. Search pg_stat_statements for matching stats.".to_string(),
        priority: NextActionPriority::Recommended,
        risk: Some(RiskLabel::Bounded),
        action_class: Some(ActionClass::TextPatternStatsSearch),
        command_template: None,
        sql_template: Some("SELECT queryid, calls, total_exec_time, mean_exec_time, rows, query FROM pg_stat_statements WHERE query ILIKE '%{normalized_sql}%' ORDER BY total_exec_time DESC LIMIT 20;".to_string()),
        required_operating_mode: Some(OperatingMode::LiveOnly),
        produces: vec!["workflow:sql_action".to_string()],
        attribution: "pg_stat_statements docs lookup".to_string(),
    }]
}

impl GuidancePayload for FindingsPayload {
    fn evaluate_rules(
        &self,
        operating_mode: OperatingMode,
        verdict: Option<Verdict>,
        config: &crate::AppConfig,
    ) -> Vec<NextAction> {
        let rules = findings_rules();
        let mut actions = Vec::new();

        for rule in rules {
            let rule_limit = config
                .guidance
                .rules
                .get(&rule.rule_id)
                .and_then(|rc| rc.limit)
                .unwrap_or(DEFAULT_RULE_LIMIT);

            if let Some(target_finding_kind) = rule.target_finding_kind {
                let mut count = 0;
                for finding in &self.findings {
                    if finding.kind != target_finding_kind {
                        continue;
                    }
                    if count >= rule_limit {
                        break;
                    }

                    let (mut status, mut reason) = crate::guidance::evaluate_rule_constraints(
                        &rule,
                        operating_mode,
                        verdict,
                        config,
                    );

                    let mut sql_preview = None;
                    let mut command = None;
                    let mut has_required_ids = true;
                    let mut missing_ids = Vec::new();

                    if rule.kind == ActionKind::RunSql {
                        if let Some(sql_temp) = &rule.sql_template {
                            let mut sql = sql_temp.clone();
                            if let Some(qf) = &finding.query_family {
                                if sql.contains("{normalized_sql}") {
                                    sql = sql.replace(
                                        "{normalized_sql}",
                                        &crate::guidance::escape_like_literal(&qf.normalized_sql),
                                    );
                                }
                                if sql.contains("{queryid}") {
                                    if let Some(qid) = &qf.queryid {
                                        sql = sql.replace("{queryid}", qid);
                                    } else {
                                        has_required_ids = false;
                                        missing_ids.push("queryid".to_string());
                                    }
                                }
                                if sql.contains("{database}") {
                                    if let Some(db) = &qf.database {
                                        sql = sql.replace(
                                            "{database}",
                                            &crate::guidance::escape_sql_literal(db),
                                        );
                                    } else {
                                        has_required_ids = false;
                                        missing_ids.push("database".to_string());
                                    }
                                }
                            } else {
                                has_required_ids = false;
                                missing_ids.push("query_family".to_string());
                            }

                            sql_preview = Some(sql.clone());
                            if has_required_ids {
                                command = Some(NextActionCommand {
                                    argv: vec![
                                        "pg-logstats".to_string(),
                                        "run-sql".to_string(),
                                        "--sql".to_string(),
                                        sql,
                                    ],
                                });
                            }
                        }
                    }

                    if status == NextActionStatus::Allowed && !has_required_ids {
                        status = NextActionStatus::OmittedNotEnoughContext;
                        reason =
                            format!("Omitted: missing required identifiers: {:?}", missing_ids);
                    }

                    if status == NextActionStatus::OmittedNotEnoughContext
                        && !config.guidance.show_omitted
                    {
                        continue;
                    }

                    let next_act = crate::guidance::build_next_action(
                        &rule,
                        status,
                        reason,
                        Some(finding.finding_id.clone()),
                        command,
                        sql_preview,
                    );
                    actions.push(next_act);
                    count += 1;
                }
            }
        }

        actions
    }
}

impl GuidancePayload for SqlActionPayload {
    fn evaluate_rules(
        &self,
        _operating_mode: OperatingMode,
        _verdict: Option<Verdict>,
        _config: &crate::AppConfig,
    ) -> Vec<NextAction> {
        vec![]
    }
}

/// Triage payload holding SQL action results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlActionPayload {
    /// The ID of the action that ran this SQL.
    pub action_id: String,
    /// The parent report that suggested this action.
    pub source_report_id: Option<String>,
    /// Executed SQL query.
    pub sql: String,
    /// Total row count returned.
    pub row_count: usize,
    /// Whether the rows were truncated to prevent bloat.
    pub truncated: bool,
    /// Column names returned.
    pub columns: Vec<String>,
    /// Matrix of row values.
    pub rows: Vec<Vec<serde_json::Value>>,
}

/// Helper function to build a SQL action triage report.
pub fn sql_action_report(
    payload: SqlActionPayload,
    operating_mode: OperatingMode,
) -> PgTriageReport<SqlActionPayload> {
    PgTriageReport {
        schema_version: PG_TRIAGE_SCHEMA_VERSION,
        workflow: ActionKind::RunSql,
        operating_mode,
        limitations: Vec::new(),
        verdict: Some(Verdict::Clear),
        verdict_reasons: Vec::new(),
        allowed_actions: None,
        blocked_actions: None,
        analysis_window: None,
        source_summary: None,
        next_actions: Vec::new(),
        report_id: None,
        session_id: None,
        parent_report_id: None,
        selected_action_id: None,
        created_at: None,
        payload,
    }
}

fn analysis_window(entries: &[LogEntry]) -> Option<AnalysisWindow> {
    let since = entries.iter().map(|entry| entry.timestamp).min()?;
    let until = entries.iter().map(|entry| entry.timestamp).max()?;

    Some(AnalysisWindow {
        since: since.to_rfc3339(),
        until: until.to_rfc3339(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Finding, FindingConfidence, FindingKind, FindingMetrics, QueryFamilyFinding, ReasonCode,
        SourceReference,
    };
    use chrono::{TimeZone, Utc};

    fn sample_findings() -> FindingSet {
        FindingSet::new(vec![Finding {
            schema_version: 1,
            finding_id: "query_family:demo".to_string(),
            kind: FindingKind::QueryFamily,
            rank: 1,
            title: "Query family with high total runtime".to_string(),
            reason: "sample".to_string(),
            reason_codes: vec![ReasonCode::HighTotalDuration],
            score: 20.0,
            query_family: Some(QueryFamilyFinding {
                query_family_id: "demo".to_string(),
                normalized_sql: "SELECT 1".to_string(),
                queryid: None,
                database: Some("appdb".to_string()),
                user: Some("app".to_string()),
                application_name: Some("api".to_string()),
                missing_attribution: Vec::new(),
            }),
            metrics: FindingMetrics {
                execution_count: 1,
                total_duration_ms: 20.0,
                avg_duration_ms: 20.0,
                max_duration_ms: 20.0,
                correlated_execution_count: 1,
                uncorrelated_execution_count: 0,
            },
            baseline: None,
            target: None,
            delta: None,
            evidence: vec![SourceReference {
                source_kind: crate::EventSourceKind::Stderr,
                record_index: 0,
            }],
            confidence: FindingConfidence::High,
            next_sql: vec![],
        }])
    }

    fn sample_entries() -> Vec<LogEntry> {
        vec![
            LogEntry::new(
                Utc.with_ymd_and_hms(2024, 1, 15, 10, 0, 0).unwrap(),
                "2001".to_string(),
                crate::LogLevel::Statement,
                "statement: SELECT 1".to_string(),
            ),
            LogEntry::new(
                Utc.with_ymd_and_hms(2024, 1, 15, 10, 0, 4).unwrap(),
                "2002".to_string(),
                crate::LogLevel::Duration,
                "duration: 20 ms".to_string(),
            ),
        ]
    }

    #[test]
    fn serializes_top_query_families_report() {
        let report = top_query_families_report(
            sample_findings(),
            &sample_entries(),
            EventSourceKind::Stderr,
        );

        let value = serde_json::to_value(&report).unwrap();

        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["workflow"], "top_query_families");
        assert_eq!(value["operating_mode"], "log_backed_only");
        assert_eq!(value["verdict"], "unknown");
        assert_eq!(value["source_summary"]["kind"], "local_stderr");
        assert_eq!(value["source_summary"]["entries_scanned"], 2);
        assert_eq!(
            value["payload"]["findings"][0]["finding_id"],
            "query_family:demo"
        );
    }
}
