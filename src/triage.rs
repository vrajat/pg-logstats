use crate::guidance::GuidancePayload;
use crate::EventSourceKind;
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

/// The interaction model required to advance a next action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NextActionType {
    /// A CLI workflow the agent can run directly.
    RunWorkflow,
    /// A bounded SQL follow-up the agent can run directly.
    RunSql,
    /// A structured decision the agent must delegate to the operator.
    PromptUser,
    /// A terminal action that ends the investigation branch.
    Stop,
}

/// Arguments to invoke a command.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NextActionCommand {
    /// Command line arguments (argv).
    pub argv: Vec<String>,
}

/// One operator choice for a delegated survey-style next action.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptUserChoice {
    /// Stable identifier for the operator choice.
    pub choice_id: String,
    /// Human-readable label shown to the operator.
    pub label: String,
    /// Short explanation of the choice and its consequence.
    pub description: String,
    /// Follow-up workflow that should run if this choice is selected.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow: Option<ActionKind>,
    /// Follow-up command template for the agent, when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<NextActionCommand>,
}

/// Structured operator survey for prompt-user next actions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptUserSurvey {
    /// The question the agent should present to the operator.
    pub question: String,
    /// The supported operator choices.
    pub choices: Vec<PromptUserChoice>,
}

/// A recommended next diagnostic action.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NextAction {
    /// The unique identifier of the action.
    pub action_id: String,
    /// The interaction type required to advance this action.
    pub action_type: NextActionType,
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
    pub target_id: Option<String>,
    /// The CLI command template to run the action.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<NextActionCommand>,
    /// Structured operator survey for delegated decisions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub survey: Option<PromptUserSurvey>,
    /// Query or command parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Vec<String>>,
    /// Risk associated with execution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk: Option<RiskLabel>,
    /// Action classification category.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_class: Option<ActionClass>,
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

pub fn workflow_slug(workflow: ActionKind) -> &'static str {
    match workflow {
        ActionKind::AgentInstall => "agent_install",
        ActionKind::Inspect => "inspect",
        ActionKind::RunningQueries => "running_queries",
        ActionKind::TopQueryFamilies => "top_query_families",
        ActionKind::SlowQueriesDiff => "slow_queries_diff",
        ActionKind::RunSql => "run_sql",
        ActionKind::CollectLogs => "collect_logs",
        ActionKind::Escalate => "escalate",
        ActionKind::Stop => "stop",
        ActionKind::Errors => "errors",
        ActionKind::TempFiles => "temp_files",
    }
}

fn push_temp_file_remedial_actions(actions: &mut Vec<NextAction>, target_id: Option<String>) {
    // 1. Indexing optimization
    actions.push(NextAction {
        action_id: "remedial.create_sort_index".to_string(),
        action_type: NextActionType::Stop,
        label: "DBA Recommendation: Create B-Tree index on sort/group columns".to_string(),
        status: NextActionStatus::Allowed,
        priority: NextActionPriority::Recommended,
        judgement_required: false,
        reason: "Create a B-Tree index on the sorting (ORDER BY) or grouping (GROUP BY) columns. This allows PostgreSQL to retrieve rows in-order using an Index Scan, completely avoiding the sort/hash step and using 0 bytes of temp files.".to_string(),
        target_id: target_id.clone(),
        command: None,
        survey: None,
        parameters: None,
        risk: None,
        action_class: None,
    });

    // 2. Select list / row width optimization
    actions.push(NextAction {
        action_id: "remedial.reduce_projection_width".to_string(),
        action_type: NextActionType::Stop,
        label: "Developer Recommendation: Select fewer columns to narrow row width".to_string(),
        status: NextActionStatus::Allowed,
        priority: NextActionPriority::Recommended,
        judgement_required: false,
        reason: "Avoid 'SELECT *' and select only the exact columns needed. Sorting narrow rows requires significantly less memory, which helps the sort fit entirely in memory without spilling to disk.".to_string(),
        target_id: target_id.clone(),
        command: None,
        survey: None,
        parameters: None,
        risk: None,
        action_class: None,
    });

    // 3. work_mem memory tuning
    actions.push(NextAction {
        action_id: "remedial.optimize_work_mem".to_string(),
        action_type: NextActionType::Stop,
        label: "DBA Recommendation: Adjust session work_mem locally".to_string(),
        status: NextActionStatus::Allowed,
        priority: NextActionPriority::Recommended,
        judgement_required: false,
        reason: "Set local/session-level work_mem before query execution (e.g. SET LOCAL work_mem = '64MB';). Avoid raising the global work_mem globally unless absolutely necessary, to prevent memory saturation/OOMs under high concurrency.".to_string(),
        target_id,
        command: None,
        survey: None,
        parameters: None,
        risk: None,
        action_class: None,
    });
}

impl GuidancePayload for SqlActionPayload {
    fn evaluate_rules(
        &self,
        operating_mode: OperatingMode,
        _verdict: Option<Verdict>,
        _config: &crate::AppConfig,
    ) -> Vec<NextAction> {
        let mut actions = Vec::new();
        let rule_id = self.action_id.split(':').next().unwrap_or(&self.action_id);

        if rule_id == "temp_file.pg_stat_database.temp_counters" {
            let temp_files_detected = self
                .insights
                .iter()
                .any(|i| i.insight_id == "temp_files_volume_detected");
            if temp_files_detected {
                // 1. Suggest checking pg_stat_statements temp block usage next
                actions.push(NextAction {
                    action_id: "temp_file.pg_stat_statements.temp_blocks".to_string(),
                    action_type: NextActionType::RunSql,
                    label: "Check pg_stat_statements temp block activity".to_string(),
                    status: if operating_mode == OperatingMode::LiveOnly || operating_mode == OperatingMode::LogBackedAndLive {
                        NextActionStatus::Allowed
                    } else {
                        NextActionStatus::BlockedByMode
                    },
                    priority: NextActionPriority::Recommended,
                    judgement_required: true,
                    reason: "Database-level temp file usage is high. Run this next to identify the exact queries writing temporary blocks in pg_stat_statements.".to_string(),
                    target_id: self.source_finding_id.clone(),
                    command: None,
                    survey: None,
                    parameters: None,
                    risk: Some(RiskLabel::Safe),
                    action_class: Some(ActionClass::StatsViewReads),
                });

                // Recommend temp file remedial actions
                push_temp_file_remedial_actions(&mut actions, self.source_finding_id.clone());
            }
        } else if rule_id == "temp_file.pg_stat_statements.temp_blocks" {
            let temp_blocks_detected = self
                .insights
                .iter()
                .any(|i| i.insight_id == "statements_writing_temp_blocks");
            if temp_blocks_detected {
                // Recommend temp file remedial actions
                push_temp_file_remedial_actions(&mut actions, self.source_finding_id.clone());
            }
        } else if rule_id == "query_family.explain"
            || rule_id == "temp_file.explain"
            || rule_id == "query_family.explain_analyze"
            || rule_id == "temp_file.explain_analyze"
        {
            let plan_disk_spill = self.insights.iter().any(|i| {
                i.insight_id == "query_plan_disk_spill_detected"
                    || i.insight_id == "explain_analyze_temp_buffers"
            });
            if plan_disk_spill {
                // Recommend temp file remedial actions
                push_temp_file_remedial_actions(&mut actions, self.source_finding_id.clone());
            }
        }

        // Always include the stop/conclude action
        actions.push(NextAction {
            action_id: "run_sql.stop.with_insight".to_string(),
            action_type: NextActionType::Stop,
            label: "Conclude live follow-up".to_string(),
            status: NextActionStatus::Allowed,
            priority: if actions.is_empty() { NextActionPriority::Recommended } else { NextActionPriority::Optional },
            judgement_required: true,
            reason: if self.insights.is_empty() {
                "No strong bounded insight was inferred from this SQL result. Use the returned rows together with the parent finding to stop or escalate outside this branch.".to_string()
            } else {
                "Use these bounded live insights to confirm or reject the hypothesis from the parent finding, then stop or escalate outside this branch.".to_string()
            },
            target_id: self.source_finding_id.clone(),
            command: None,
            survey: None,
            parameters: None,
            risk: None,
            action_class: None,
        });

        actions
    }
}

/// Confidence level for a bounded SQL-derived insight.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SqlInsightConfidence {
    /// High-confidence interpretation from specific fields.
    High,
    /// Medium-confidence interpretation from partial evidence.
    Medium,
}

/// A bounded interpretation derived from a built-in SQL action result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SqlActionInsight {
    /// Stable identifier for the interpreted pattern.
    pub insight_id: String,
    /// Short operator-facing summary.
    pub label: String,
    /// Confidence for this interpretation.
    pub confidence: SqlInsightConfidence,
    /// Evidence-based explanation of the pattern.
    pub reason: String,
}

/// Triage payload holding SQL action results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlActionPayload {
    /// The ID of the action that ran this SQL.
    pub action_id: String,
    /// The parent report that suggested this action.
    pub source_report_id: Option<String>,
    /// The source finding that this live follow-up was derived from.
    pub source_finding_id: Option<String>,
    /// Optional bounded interpretations of the SQL result.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub insights: Vec<SqlActionInsight>,
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
        parent_report_id: None,
        selected_action_id: None,
        created_at: None,
        payload,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppConfig;
    use crate::{
        Finding, FindingConfidence, FindingKind, FindingMetrics, FindingSet, FindingsPayload,
        LogEntry, QueryFamilyFinding, ReasonCode, SourceReference, TempFileFinding,
    };
    use chrono::{TimeZone, Utc};

    fn sample_findings() -> FindingSet {
        FindingSet::new(vec![Finding {
            id: "demo".to_string(),
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
            error_class: None,
            temp_file: None,
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

    fn sample_report_payload() -> FindingsPayload {
        FindingsPayload {
            findings: sample_findings().findings,
        }
    }

    #[test]
    fn serializes_top_query_families_report() {
        let report = crate::findings::top_query_families_report(
            sample_findings(),
            &sample_entries(),
            EventSourceKind::Stderr,
        );

        let value = serde_json::to_value(&report).unwrap();

        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["workflow"], "top_query_families");
        assert_eq!(value["operating_mode"], "log_backed_only");
        assert!(value.get("verdict").is_none());
        assert_eq!(
            value["limitations"],
            serde_json::json!(["live_database_checks_unavailable"])
        );
        assert_eq!(value["source_summary"]["kind"], "local_stderr");
        assert_eq!(value["source_summary"]["entries_scanned"], 2);
        assert_eq!(value["payload"]["findings"][0]["id"], "demo");
    }

    #[test]
    fn top_query_families_offline_adds_prompt_user_follow_up() {
        let mut report = crate::findings::top_query_families_report(
            sample_findings(),
            &sample_entries(),
            EventSourceKind::Stderr,
        );

        crate::populate_next_actions(&mut report, &AppConfig::default());

        let action = report
            .next_actions
            .iter()
            .find(|action| action.action_id == "workspace.prompt_user.enable_live_follow_up")
            .unwrap();
        assert_eq!(action.action_type, NextActionType::PromptUser);
        assert_eq!(action.status, NextActionStatus::Allowed);
        assert!(action.reason.contains("[database].dsn in config.toml"));
        assert!(action
            .survey
            .as_ref()
            .unwrap()
            .choices
            .first()
            .unwrap()
            .description
            .contains("PG_LOGSTATS_DATABASE_URL"));
        assert_eq!(
            action
                .survey
                .as_ref()
                .unwrap()
                .choices
                .first()
                .unwrap()
                .command
                .as_ref()
                .unwrap()
                .argv,
            vec!["pg-logstats", "inspect"]
        );
    }

    #[test]
    fn top_query_families_live_omits_prompt_user_follow_up() {
        let mut report = crate::findings::top_query_families_report(
            sample_findings(),
            &sample_entries(),
            EventSourceKind::Stderr,
        );
        report.operating_mode = OperatingMode::LogBackedAndLive;
        report.verdict = Some(Verdict::Clear);

        crate::populate_next_actions(&mut report, &AppConfig::default());

        assert!(report
            .next_actions
            .iter()
            .all(|action| action.action_type != NextActionType::PromptUser));
    }

    #[test]
    fn query_family_rules_emit_activity_action_and_omit_queryid_without_context() {
        let actions = sample_report_payload().evaluate_rules(
            OperatingMode::LogBackedAndLive,
            Some(Verdict::Clear),
            &AppConfig::default(),
        );

        let activity = actions
            .iter()
            .find(|a| a.action_id == "query_family.pg_stat_activity.by_dimensions:demo")
            .unwrap();
        assert_eq!(activity.status, NextActionStatus::Allowed);
        assert_eq!(activity.risk, Some(RiskLabel::Safe));
        assert_eq!(activity.action_type, NextActionType::RunSql);
        assert_eq!(activity.target_id.as_deref(), Some("demo"));
        assert_eq!(
            activity.command.as_ref().unwrap().argv,
            vec!["pg-logstats", "run-sql"]
        );

        let queryid = actions
            .iter()
            .find(|a| a.action_id == "query_family.pg_stat_statements.by_queryid:demo")
            .unwrap();
        assert_eq!(queryid.status, NextActionStatus::OmittedNotEnoughContext);
    }

    #[test]
    fn query_family_rules_emit_exact_queryid() {
        let mut payload = sample_report_payload();
        payload.findings[0].query_family.as_mut().unwrap().queryid = Some("918273645".to_string());

        let actions = payload.evaluate_rules(
            OperatingMode::LogBackedAndLive,
            Some(Verdict::Clear),
            &AppConfig::default(),
        );

        let queryid = actions
            .iter()
            .find(|a| a.action_id == "query_family.pg_stat_statements.by_queryid:demo")
            .unwrap();
        assert_eq!(queryid.status, NextActionStatus::Allowed);
        assert_eq!(queryid.action_type, NextActionType::RunSql);
        assert_eq!(queryid.target_id.as_deref(), Some("demo"));
    }

    #[test]
    fn query_family_rules_escape_activity_literals() {
        let mut payload = sample_report_payload();
        let qf = payload.findings[0].query_family.as_mut().unwrap();
        qf.application_name = Some("api's worker".to_string());

        let actions = payload.evaluate_rules(
            OperatingMode::LogBackedAndLive,
            Some(Verdict::Clear),
            &AppConfig::default(),
        );

        let activity = actions
            .iter()
            .find(|a| a.action_id == "query_family.pg_stat_activity.by_dimensions:demo")
            .unwrap();
        assert_eq!(activity.action_type, NextActionType::RunSql);
        assert_eq!(activity.target_id.as_deref(), Some("demo"));
    }

    #[test]
    fn query_family_rules_emit_explain_actions() {
        let payload = sample_report_payload();

        let actions = payload.evaluate_rules(
            OperatingMode::LogBackedAndLive,
            Some(Verdict::Clear),
            &AppConfig::default(),
        );

        let explain = actions
            .iter()
            .find(|a| a.action_id == "query_family.explain:demo")
            .unwrap();
        assert_eq!(explain.status, NextActionStatus::Allowed);
        assert_eq!(explain.action_type, NextActionType::RunSql);
        assert_eq!(
            explain.action_class,
            Some(ActionClass::ExplainWithoutAnalyze)
        );

        let explain_analyze = actions
            .iter()
            .find(|a| a.action_id == "query_family.explain_analyze:demo")
            .unwrap();
        assert_eq!(explain_analyze.status, NextActionStatus::BlockedByVerdict);
        assert_eq!(explain_analyze.action_type, NextActionType::RunSql);
        assert_eq!(
            explain_analyze.action_class,
            Some(ActionClass::ExplainAnalyze)
        );
    }

    #[test]
    fn temp_file_rules_emit_explain_actions() {
        let mut findings = sample_findings();
        findings.findings[0].kind = FindingKind::TempFile;
        findings.findings[0].query_family = None;
        findings.findings[0].temp_file = Some(TempFileFinding {
            query_family_id: Some("demo".to_string()),
            normalized_sql: Some("SELECT 1".to_string()),
            database: Some("appdb".to_string()),
            user: Some("app".to_string()),
            application_name: Some("api".to_string()),
            largest_observed_bytes: 5000000,
            temp_file_count: 1,
            total_bytes: 5000000,
        });

        let payload = FindingsPayload {
            findings: findings.findings,
        };

        let actions = payload.evaluate_rules(
            OperatingMode::LogBackedAndLive,
            Some(Verdict::Clear),
            &AppConfig::default(),
        );

        let explain = actions
            .iter()
            .find(|a| a.action_id == "temp_file.explain:demo")
            .unwrap();
        assert_eq!(explain.status, NextActionStatus::Allowed);
        assert_eq!(explain.action_type, NextActionType::RunSql);
        assert_eq!(
            explain.action_class,
            Some(ActionClass::ExplainWithoutAnalyze)
        );

        let explain_analyze = actions
            .iter()
            .find(|a| a.action_id == "temp_file.explain_analyze:demo")
            .unwrap();
        assert_eq!(explain_analyze.status, NextActionStatus::BlockedByVerdict);
        assert_eq!(explain_analyze.action_type, NextActionType::RunSql);
        assert_eq!(
            explain_analyze.action_class,
            Some(ActionClass::ExplainAnalyze)
        );
    }
}
