//! Structured findings for investigation-oriented output.

use crate::guidance::{build_next_action, evaluate_rule_constraints, GuidancePayload};
use crate::triage::{
    ActionClass, ActionKind, AnalysisWindow, NextAction, NextActionCommand, NextActionPriority,
    NextActionStatus, OperatingMode, PgTriageReport, RiskLabel, SourceSummary, SourceSummaryKind,
    Verdict, PG_TRIAGE_SCHEMA_VERSION,
};
use crate::{
    CorrelationConfidence, EventSourceKind, LogEntry, QueryExecution, QueryFamilyIdentity,
    RuleDefinition, RuleId, SourceReference, DEFAULT_RULE_LIMIT,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const FINDING_SCHEMA_VERSION: u32 = 1;

/// Collection wrapper for versioned finding output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingSet {
    pub schema_version: u32,
    pub findings: Vec<Finding>,
}

impl FindingSet {
    pub fn new(findings: Vec<Finding>) -> Self {
        Self {
            schema_version: FINDING_SCHEMA_VERSION,
            findings,
        }
    }
}

/// Machine-readable investigation finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub schema_version: u32,
    pub finding_id: String,
    pub kind: FindingKind,
    pub rank: usize,
    pub title: String,
    pub reason: String,
    pub reason_codes: Vec<ReasonCode>,
    pub score: f64,
    pub query_family: Option<QueryFamilyFinding>,
    pub metrics: FindingMetrics,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baseline: Option<ComparisonMetrics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<ComparisonMetrics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<DeltaMetrics>,
    pub evidence: Vec<SourceReference>,
    pub confidence: FindingConfidence,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_class: Option<ErrorClassFinding>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temp_file: Option<TempFileFinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorClassFinding {
    pub sqlstate: Option<String>,
    pub normalized_error: String,
    pub database: Option<String>,
    pub user: Option<String>,
    pub application_name: Option<String>,
    pub error_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TempFileFinding {
    pub query_family_id: Option<String>,
    pub normalized_sql: Option<String>,
    pub database: Option<String>,
    pub user: Option<String>,
    pub application_name: Option<String>,
    pub largest_observed_bytes: u64,
    pub temp_file_count: u64,
    pub total_bytes: u64,
}

/// Finding family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingKind {
    QueryFamily,
    SlowQueryRegression,
    ErrorClass,
    TempFile,
}

/// Compact reason codes intended for downstream tooling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasonCode {
    HighTotalDuration,
    HighMaxDuration,
    CorrelatedDuration,
    PartialCorrelation,
    AbsentInBaseline,
    P95Regressed,
    RuntimeContributionIncreased,
    MeetsEligibilityThresholds,
}

/// Overall confidence for ranking and evidence reconstruction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingConfidence {
    High,
    Medium,
    Low,
}

/// Query-family dimensions included in query-family findings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryFamilyFinding {
    pub query_family_id: String,
    /// The normalized query-family text used for grouping and evidence display.
    pub normalized_sql: String,
    pub queryid: Option<String>,
    pub database: Option<String>,
    pub user: Option<String>,
    pub application_name: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub missing_attribution: Vec<AttributionField>,
}

/// Attribution dimensions that were unavailable in the supporting log evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttributionField {
    ApplicationName,
    Database,
    User,
}

impl From<&QueryFamilyIdentity> for QueryFamilyFinding {
    fn from(identity: &QueryFamilyIdentity) -> Self {
        let mut missing_attribution = Vec::new();
        if identity.application_name.is_none() {
            missing_attribution.push(AttributionField::ApplicationName);
        }
        if identity.database.is_none() {
            missing_attribution.push(AttributionField::Database);
        }
        if identity.user.is_none() {
            missing_attribution.push(AttributionField::User);
        }

        Self {
            query_family_id: identity.family_id.clone(),
            normalized_sql: identity.normalized_sql.clone(),
            queryid: identity.queryid.clone(),
            database: identity.database.clone(),
            user: identity.user.clone(),
            application_name: identity.application_name.clone(),
            missing_attribution,
        }
    }
}

/// Triage payload holding serialized findings for report-shaped workflows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingsPayload {
    /// List of diagnostic findings.
    pub findings: Vec<Finding>,
}

/// Summary metrics attached to a finding.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct FindingMetrics {
    pub execution_count: u64,
    pub total_duration_ms: f64,
    pub avg_duration_ms: f64,
    pub max_duration_ms: f64,
    pub correlated_execution_count: u64,
    pub uncorrelated_execution_count: u64,
}

/// Window-specific metrics used by baseline-vs-target findings.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ComparisonMetrics {
    pub execution_count: u64,
    pub total_duration_ms: f64,
    pub avg_duration_ms: f64,
    pub p95_duration_ms: f64,
    pub max_duration_ms: f64,
}

/// Deterministic deltas between target and baseline windows.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DeltaMetrics {
    pub execution_count: i64,
    pub total_duration_ms: f64,
    pub avg_duration_ms: f64,
    pub p95_duration_ms: f64,
    pub max_duration_ms: f64,
}

/// Thresholds for baseline-vs-target query family diffing.
#[derive(Debug, Clone, Copy)]
pub struct SlowQueryDiffOptions {
    pub limit: usize,
    pub min_target_count: u64,
    pub min_target_total_ms: f64,
    pub min_p95_delta_ms: f64,
}

impl Default for SlowQueryDiffOptions {
    fn default() -> Self {
        Self {
            limit: 10,
            min_target_count: 1,
            min_target_total_ms: 0.0,
            min_p95_delta_ms: 0.0,
        }
    }
}

/// Build a top-query-families triage report from ranked findings and source
/// metadata.
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
            kind: SourceSummaryKind::from(source_kind),
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

/// Built-in rule definitions for findings-backed workflow follow-up actions.
pub fn findings_rules() -> Vec<RuleDefinition> {
    vec![
        RuleDefinition {
            rule_id: RuleId::QueryFamilyPgStatStatementsByQueryId,
            emitted_action_id: RuleId::QueryFamilyPgStatStatementsByQueryId,
            kind: ActionKind::RunSql,
            target_workflow: ActionKind::TopQueryFamilies,
            target_finding_kind: Some(FindingKind::QueryFamily),
            destination_workflow: Some(ActionKind::RunSql),
            required_identifiers: vec!["queryid".to_string()],
            label: "Inspect statement statistics for the exact query family".to_string(),
            reason: "The finding includes queryid, so this is an exact stats-view lookup."
                .to_string(),
            priority: NextActionPriority::Recommended,
            risk: Some(RiskLabel::Safe),
            action_class: Some(ActionClass::StatsViewReads),
            command_template: None,
            sql_template: None,
            required_operating_mode: Some(OperatingMode::LiveOnly),
            produces: vec!["workflow:sql_action".to_string()],
            attribution: "PostgreSQL pg_stat_statements exact queryid lookup".to_string(),
        },
        RuleDefinition {
            rule_id: RuleId::QueryFamilyPgStatActivityByDimensions,
            emitted_action_id: RuleId::QueryFamilyPgStatActivityByDimensions,
            kind: ActionKind::RunSql,
            target_workflow: ActionKind::TopQueryFamilies,
            target_finding_kind: Some(FindingKind::QueryFamily),
            destination_workflow: Some(ActionKind::RunSql),
            required_identifiers: vec!["database|user|application_name".to_string()],
            label: "Find current active sessions for the same query-family dimensions"
                .to_string(),
            reason: "The finding includes database, user, or application attribution that can bound pg_stat_activity."
                .to_string(),
            priority: NextActionPriority::Optional,
            risk: Some(RiskLabel::Bounded),
            action_class: Some(ActionClass::BoundedActivityQueries),
            command_template: None,
            sql_template: None,
            required_operating_mode: Some(OperatingMode::LiveOnly),
            produces: vec!["workflow:sql_action".to_string()],
            attribution: "PostgreSQL pg_stat_activity lookup by app, database, and user"
                .to_string(),
        },
        RuleDefinition {
            rule_id: RuleId::ErrorClassPgStatActivityByDimensions,
            emitted_action_id: RuleId::ErrorClassPgStatActivityByDimensions,
            kind: ActionKind::RunSql,
            target_workflow: ActionKind::Errors,
            target_finding_kind: Some(FindingKind::ErrorClass),
            destination_workflow: Some(ActionKind::RunSql),
            required_identifiers: vec!["database|user|application_name".to_string()],
            label: "Find current active sessions for the same error-class dimensions"
                .to_string(),
            reason: "The finding includes database, user, or application attribution that can bound pg_stat_activity."
                .to_string(),
            priority: NextActionPriority::Recommended,
            risk: Some(RiskLabel::Bounded),
            action_class: Some(ActionClass::BoundedActivityQueries),
            command_template: None,
            sql_template: None,
            required_operating_mode: Some(OperatingMode::LiveOnly),
            produces: vec!["workflow:sql_action".to_string()],
            attribution: "PostgreSQL pg_stat_activity lookup by app, database, and user for error class"
                .to_string(),
        },
        RuleDefinition {
            rule_id: RuleId::TempFilePgStatDatabaseTempCounters,
            emitted_action_id: RuleId::TempFilePgStatDatabaseTempCounters,
            kind: ActionKind::RunSql,
            target_workflow: ActionKind::TempFiles,
            target_finding_kind: Some(FindingKind::TempFile),
            destination_workflow: Some(ActionKind::RunSql),
            required_identifiers: vec!["database".to_string()],
            label: "Check database temp counters in pg_stat_database"
                .to_string(),
            reason: "The finding includes database attribution, so we can check total temp files/bytes for this database."
                .to_string(),
            priority: NextActionPriority::Recommended,
            risk: Some(RiskLabel::Safe),
            action_class: Some(ActionClass::StatsViewReads),
            command_template: None,
            sql_template: None,
            required_operating_mode: Some(OperatingMode::LiveOnly),
            produces: vec!["workflow:sql_action".to_string()],
            attribution: "PostgreSQL pg_stat_database temp counters check"
                .to_string(),
        },
        RuleDefinition {
            rule_id: RuleId::TempFilePgStatStatementsTempBlocks,
            emitted_action_id: RuleId::TempFilePgStatStatementsTempBlocks,
            kind: ActionKind::RunSql,
            target_workflow: ActionKind::TempFiles,
            target_finding_kind: Some(FindingKind::TempFile),
            destination_workflow: Some(ActionKind::RunSql),
            required_identifiers: vec![],
            label: "Check pg_stat_statements temp block activity"
                .to_string(),
            reason: "Check overall temp block read/write activity in pg_stat_statements."
                .to_string(),
            priority: NextActionPriority::Recommended,
            risk: Some(RiskLabel::Safe),
            action_class: Some(ActionClass::StatsViewReads),
            command_template: None,
            sql_template: None,
            required_operating_mode: Some(OperatingMode::LiveOnly),
            produces: vec!["workflow:sql_action".to_string()],
            attribution: "PostgreSQL pg_stat_statements temp block reads/writes check"
                .to_string(),
        },
    ]
}

fn query_family_queryid_sql() -> String {
    "SELECT queryid, calls, total_exec_time, mean_exec_time, min_exec_time, max_exec_time, rows, shared_blks_hit, shared_blks_read, temp_blks_read, temp_blks_written, query FROM pg_stat_statements WHERE queryid = $1;".to_string()
}

fn query_family_activity_sql(
    database: Option<&str>,
    user: Option<&str>,
    application_name: Option<&str>,
) -> Option<String> {
    let mut predicates = Vec::new();
    if let Some(database) = database {
        predicates.push(format!(
            "datname = '{}'",
            crate::guidance::escape_sql_literal(database)
        ));
    }
    if let Some(user) = user {
        predicates.push(format!(
            "usename = '{}'",
            crate::guidance::escape_sql_literal(user)
        ));
    }
    if let Some(application_name) = application_name {
        predicates.push(format!(
            "application_name = '{}'",
            crate::guidance::escape_sql_literal(application_name)
        ));
    }

    if predicates.is_empty() {
        return None;
    }

    Some(format!(
        "SELECT pid, usename, datname, application_name, state, wait_event_type, wait_event, query_start, query_id, query FROM pg_stat_activity WHERE {} AND state <> 'idle' ORDER BY query_start DESC NULLS LAST LIMIT 20;",
        predicates.join(" AND ")
    ))
}

impl GuidancePayload for FindingsPayload {
    fn evaluate_rules(
        &self,
        operating_mode: OperatingMode,
        verdict: Option<Verdict>,
        config: &crate::AppConfig,
    ) -> Vec<NextAction> {
        let mut actions = Vec::new();

        for rule in findings_rules() {
            let Some(target_finding_kind) = rule.target_finding_kind else {
                continue;
            };

            let rule_limit = config
                .guidance
                .rules
                .get(&rule.rule_id)
                .and_then(|rc| rc.limit)
                .unwrap_or(DEFAULT_RULE_LIMIT);

            let matching_findings = self
                .findings
                .iter()
                .filter(|finding| finding.kind == target_finding_kind)
                .take(rule_limit);

            for finding in matching_findings {
                let mut resolved_rule = rule.clone();
                let mut sql_preview = None;
                let mut command = None;
                let mut missing_ids = Vec::new();

                match rule.rule_id {
                    RuleId::QueryFamilyPgStatStatementsByQueryId => {
                        if let Some(qf) = &finding.query_family {
                            if qf.queryid.is_some() {
                                let sql = query_family_queryid_sql();
                                sql_preview = Some(sql);
                                command = Some(NextActionCommand {
                                    argv: vec!["pg-logstats".to_string(), "run-sql".to_string()],
                                });
                            } else {
                                missing_ids.push("queryid".to_string());
                            }
                        } else {
                            missing_ids.push("query_family".to_string());
                        }
                    }
                    RuleId::QueryFamilyPgStatActivityByDimensions => {
                        if let Some(qf) = &finding.query_family {
                            let sql = query_family_activity_sql(
                                qf.database.as_deref(),
                                qf.user.as_deref(),
                                qf.application_name.as_deref(),
                            );
                            if let Some(sql) = sql {
                                resolved_rule.risk = Some(if qf.application_name.is_some() {
                                    RiskLabel::Safe
                                } else {
                                    RiskLabel::Bounded
                                });
                                sql_preview = Some(sql);
                                command = Some(NextActionCommand {
                                    argv: vec!["pg-logstats".to_string(), "run-sql".to_string()],
                                });
                            } else {
                                missing_ids.push("database|user|application_name".to_string());
                            }
                        } else {
                            missing_ids.push("query_family".to_string());
                        }
                    }
                    RuleId::ErrorClassPgStatActivityByDimensions => {
                        if let Some(ec) = &finding.error_class {
                            let sql = query_family_activity_sql(
                                ec.database.as_deref(),
                                ec.user.as_deref(),
                                ec.application_name.as_deref(),
                            );
                            if let Some(sql) = sql {
                                resolved_rule.risk = Some(if ec.application_name.is_some() {
                                    RiskLabel::Safe
                                } else {
                                    RiskLabel::Bounded
                                });
                                sql_preview = Some(sql);
                                command = Some(NextActionCommand {
                                    argv: vec!["pg-logstats".to_string(), "run-sql".to_string()],
                                });
                            } else {
                                missing_ids.push("database|user|application_name".to_string());
                            }
                        } else {
                            missing_ids.push("error_class".to_string());
                        }
                    }
                    RuleId::TempFilePgStatDatabaseTempCounters => {
                        if let Some(tf) = &finding.temp_file {
                            if let Some(db) = &tf.database {
                                sql_preview = Some(format!(
                                    "SELECT datname, temp_files, temp_bytes FROM pg_stat_database WHERE datname = '{}';",
                                    crate::guidance::escape_sql_literal(db)
                                ));
                                command = Some(NextActionCommand {
                                    argv: vec!["pg-logstats".to_string(), "run-sql".to_string()],
                                });
                            } else {
                                missing_ids.push("database".to_string());
                            }
                        } else {
                            missing_ids.push("temp_file".to_string());
                        }
                    }
                    RuleId::TempFilePgStatStatementsTempBlocks => {
                        sql_preview = Some("SELECT queryid, calls, total_exec_time, temp_blks_read, temp_blks_written, query FROM pg_stat_statements WHERE temp_blks_read > 0 OR temp_blks_written > 0 ORDER BY temp_blks_read + temp_blks_written DESC LIMIT 20;".to_string());
                        command = Some(NextActionCommand {
                            argv: vec!["pg-logstats".to_string(), "run-sql".to_string()],
                        });
                    }
                    _ => {}
                }

                let (mut status, mut reason) =
                    evaluate_rule_constraints(&resolved_rule, operating_mode, verdict, config);

                if status == NextActionStatus::Allowed && !missing_ids.is_empty() {
                    status = NextActionStatus::OmittedNotEnoughContext;
                    reason = format!("Omitted: missing required identifiers: {:?}", missing_ids);
                }

                if status == NextActionStatus::OmittedNotEnoughContext
                    && !config.guidance.show_omitted
                {
                    continue;
                }

                actions.push(build_next_action(
                    &resolved_rule,
                    status,
                    reason,
                    Some(finding.finding_id.clone()),
                    command,
                    sql_preview,
                ));
            }
        }

        actions
    }
}

#[derive(Debug, Clone)]
struct QueryFamilyAccumulator {
    identity: QueryFamilyIdentity,
    execution_count: u64,
    total_duration_ms: f64,
    max_duration_ms: f64,
    correlated_execution_count: u64,
    uncorrelated_execution_count: u64,
    evidence: Vec<SourceReference>,
}

impl QueryFamilyAccumulator {
    fn new(identity: QueryFamilyIdentity) -> Self {
        Self {
            identity,
            execution_count: 0,
            total_duration_ms: 0.0,
            max_duration_ms: 0.0,
            correlated_execution_count: 0,
            uncorrelated_execution_count: 0,
            evidence: Vec::new(),
        }
    }

    fn add_execution(&mut self, execution: &QueryExecution) {
        self.execution_count += 1;
        if let Some(duration_ms) = execution.duration_ms {
            self.total_duration_ms += duration_ms;
            self.max_duration_ms = self.max_duration_ms.max(duration_ms);
        }

        match execution.confidence {
            CorrelationConfidence::Exact => self.correlated_execution_count += 1,
            CorrelationConfidence::StatementOnly => self.uncorrelated_execution_count += 1,
        }

        for source in &execution.evidence {
            if self.evidence.len() >= 3 {
                break;
            }
            self.evidence.push(source.clone());
        }
    }

    fn into_finding(self, rank: usize) -> Finding {
        let avg_duration_ms = if self.execution_count == 0 {
            0.0
        } else {
            self.total_duration_ms / self.execution_count as f64
        };

        let metrics = FindingMetrics {
            execution_count: self.execution_count,
            total_duration_ms: self.total_duration_ms,
            avg_duration_ms,
            max_duration_ms: self.max_duration_ms,
            correlated_execution_count: self.correlated_execution_count,
            uncorrelated_execution_count: self.uncorrelated_execution_count,
        };

        let confidence = if self.uncorrelated_execution_count == 0 {
            FindingConfidence::High
        } else if self.correlated_execution_count > 0 {
            FindingConfidence::Medium
        } else {
            FindingConfidence::Low
        };

        let mut reason_codes = vec![ReasonCode::HighTotalDuration, ReasonCode::HighMaxDuration];
        if self.correlated_execution_count > 0 {
            reason_codes.push(ReasonCode::CorrelatedDuration);
        }
        if self.uncorrelated_execution_count > 0 {
            reason_codes.push(ReasonCode::PartialCorrelation);
        }

        Finding {
            schema_version: FINDING_SCHEMA_VERSION,
            finding_id: format!("query_family:{}", self.identity.family_id),
            kind: FindingKind::QueryFamily,
            rank,
            title: "Query family with high total runtime".to_string(),
            reason: format!(
                "{} executions contributed {:.3} ms total runtime; max execution was {:.3} ms",
                metrics.execution_count, metrics.total_duration_ms, metrics.max_duration_ms
            ),
            reason_codes,
            score: metrics.total_duration_ms,
            query_family: Some(QueryFamilyFinding::from(&self.identity)),
            metrics,
            baseline: None,
            target: None,
            delta: None,
            evidence: self.evidence,
            confidence,
            error_class: None,
            temp_file: None,
        }
    }
}

/// Build ranked query-family findings from correlated executions.
pub fn query_family_findings(executions: &[QueryExecution], limit: usize) -> FindingSet {
    let mut by_family: HashMap<String, QueryFamilyAccumulator> = HashMap::new();

    for execution in executions {
        let family_id = execution.query_family.family_id.clone();
        by_family
            .entry(family_id)
            .or_insert_with(|| QueryFamilyAccumulator::new(execution.query_family.clone()))
            .add_execution(execution);
    }

    let mut accumulators: Vec<_> = by_family.into_values().collect();
    accumulators.sort_by(|a, b| {
        b.total_duration_ms
            .partial_cmp(&a.total_duration_ms)
            .unwrap()
            .then_with(|| a.identity.family_id.cmp(&b.identity.family_id))
    });

    let findings = accumulators
        .into_iter()
        .take(limit)
        .enumerate()
        .map(|(index, accumulator)| accumulator.into_finding(index + 1))
        .collect();

    FindingSet::new(findings)
}

#[derive(Debug, Clone)]
struct DiffAccumulator {
    identity: QueryFamilyIdentity,
    durations: Vec<f64>,
    correlated_execution_count: u64,
    uncorrelated_execution_count: u64,
    evidence: Vec<SourceReference>,
}

#[derive(Debug, Clone)]
struct DiffCandidate {
    score: f64,
    accumulator: DiffAccumulator,
    baseline: ComparisonMetrics,
    target: ComparisonMetrics,
    delta: DeltaMetrics,
    absent_in_baseline: bool,
    p95_regressed: bool,
    runtime_increased: bool,
}

impl DiffAccumulator {
    fn new(identity: QueryFamilyIdentity) -> Self {
        Self {
            identity,
            durations: Vec::new(),
            correlated_execution_count: 0,
            uncorrelated_execution_count: 0,
            evidence: Vec::new(),
        }
    }

    fn add_execution(&mut self, execution: &QueryExecution) {
        if let Some(duration_ms) = execution.duration_ms {
            self.durations.push(duration_ms);
        }

        match execution.confidence {
            CorrelationConfidence::Exact => self.correlated_execution_count += 1,
            CorrelationConfidence::StatementOnly => self.uncorrelated_execution_count += 1,
        }

        for source in &execution.evidence {
            if self.evidence.len() >= 3 {
                break;
            }
            self.evidence.push(source.clone());
        }
    }

    fn comparison_metrics(&self) -> ComparisonMetrics {
        comparison_metrics(&self.durations)
    }
}

/// Build baseline-vs-target slow query findings from correlated executions.
pub fn slow_query_diff_findings(
    baseline: &[QueryExecution],
    target: &[QueryExecution],
    options: SlowQueryDiffOptions,
) -> FindingSet {
    let baseline_by_family = diff_accumulators_by_family(baseline);
    let target_by_family = diff_accumulators_by_family(target);
    let mut candidates = Vec::new();

    for (family_id, target_accumulator) in target_by_family {
        let target_metrics = target_accumulator.comparison_metrics();
        if target_metrics.execution_count < options.min_target_count
            || target_metrics.total_duration_ms < options.min_target_total_ms
        {
            continue;
        }

        let baseline_metrics = baseline_by_family
            .get(&family_id)
            .map(|accumulator| accumulator.comparison_metrics())
            .unwrap_or_else(|| comparison_metrics(&[]));
        let delta = DeltaMetrics {
            execution_count: target_metrics.execution_count as i64
                - baseline_metrics.execution_count as i64,
            total_duration_ms: target_metrics.total_duration_ms
                - baseline_metrics.total_duration_ms,
            avg_duration_ms: target_metrics.avg_duration_ms - baseline_metrics.avg_duration_ms,
            p95_duration_ms: target_metrics.p95_duration_ms - baseline_metrics.p95_duration_ms,
            max_duration_ms: target_metrics.max_duration_ms - baseline_metrics.max_duration_ms,
        };

        let absent_in_baseline = baseline_metrics.execution_count == 0;
        let p95_regressed = delta.p95_duration_ms >= options.min_p95_delta_ms
            && target_metrics.p95_duration_ms > baseline_metrics.p95_duration_ms;
        let runtime_increased = delta.total_duration_ms > 0.0;

        if !absent_in_baseline && !p95_regressed && !runtime_increased {
            continue;
        }

        let score = if absent_in_baseline {
            target_metrics.total_duration_ms + target_metrics.p95_duration_ms
        } else {
            delta.total_duration_ms.max(0.0) + delta.p95_duration_ms.max(0.0)
        };

        candidates.push(DiffCandidate {
            score,
            accumulator: target_accumulator,
            baseline: baseline_metrics,
            target: target_metrics,
            delta,
            absent_in_baseline,
            p95_regressed,
            runtime_increased,
        });
    }

    candidates.sort_by(|a, b| {
        b.score.partial_cmp(&a.score).unwrap().then_with(|| {
            a.accumulator
                .identity
                .family_id
                .cmp(&b.accumulator.identity.family_id)
        })
    });

    let findings = candidates
        .into_iter()
        .take(options.limit)
        .enumerate()
        .map(|(index, candidate)| diff_finding(index + 1, candidate))
        .collect();

    FindingSet::new(findings)
}

fn diff_accumulators_by_family(executions: &[QueryExecution]) -> HashMap<String, DiffAccumulator> {
    let mut by_family = HashMap::new();

    for execution in executions {
        let family_id = execution.query_family.family_id.clone();
        by_family
            .entry(family_id)
            .or_insert_with(|| DiffAccumulator::new(execution.query_family.clone()))
            .add_execution(execution);
    }

    by_family
}

fn comparison_metrics(durations: &[f64]) -> ComparisonMetrics {
    if durations.is_empty() {
        return ComparisonMetrics {
            execution_count: 0,
            total_duration_ms: 0.0,
            avg_duration_ms: 0.0,
            p95_duration_ms: 0.0,
            max_duration_ms: 0.0,
        };
    }

    let mut sorted = durations.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let total_duration_ms = sorted.iter().sum::<f64>();
    let execution_count = sorted.len() as u64;
    let p95_index = (sorted.len() as f64 * 0.95) as usize;

    ComparisonMetrics {
        execution_count,
        total_duration_ms,
        avg_duration_ms: total_duration_ms / execution_count as f64,
        p95_duration_ms: sorted[p95_index.min(sorted.len() - 1)],
        max_duration_ms: *sorted.last().unwrap(),
    }
}

fn diff_finding(rank: usize, candidate: DiffCandidate) -> Finding {
    let DiffCandidate {
        score,
        accumulator,
        baseline,
        target,
        delta,
        absent_in_baseline,
        p95_regressed,
        runtime_increased,
    } = candidate;

    let mut reason_codes = vec![ReasonCode::MeetsEligibilityThresholds];
    let mut reason_parts = Vec::new();

    if absent_in_baseline {
        reason_codes.push(ReasonCode::AbsentInBaseline);
        reason_parts.push("absent in baseline".to_string());
    }
    if p95_regressed {
        reason_codes.push(ReasonCode::P95Regressed);
        reason_parts.push(format!("p95 increased by {:.3} ms", delta.p95_duration_ms));
    }
    if runtime_increased {
        reason_codes.push(ReasonCode::RuntimeContributionIncreased);
        reason_parts.push(format!(
            "total runtime increased by {:.3} ms",
            delta.total_duration_ms
        ));
    }
    if accumulator.correlated_execution_count > 0 {
        reason_codes.push(ReasonCode::CorrelatedDuration);
    }
    if accumulator.uncorrelated_execution_count > 0 {
        reason_codes.push(ReasonCode::PartialCorrelation);
    }

    let confidence = if accumulator.uncorrelated_execution_count == 0 {
        FindingConfidence::High
    } else if accumulator.correlated_execution_count > 0 {
        FindingConfidence::Medium
    } else {
        FindingConfidence::Low
    };

    let metrics = FindingMetrics {
        execution_count: target.execution_count,
        total_duration_ms: target.total_duration_ms,
        avg_duration_ms: target.avg_duration_ms,
        max_duration_ms: target.max_duration_ms,
        correlated_execution_count: accumulator.correlated_execution_count,
        uncorrelated_execution_count: accumulator.uncorrelated_execution_count,
    };

    Finding {
        schema_version: FINDING_SCHEMA_VERSION,
        finding_id: format!("slow_query_diff:{}", accumulator.identity.family_id),
        kind: FindingKind::SlowQueryRegression,
        rank,
        title: "Query family regressed in target window".to_string(),
        reason: reason_parts.join("; "),
        reason_codes,
        score,
        query_family: Some(QueryFamilyFinding::from(&accumulator.identity)),
        metrics,
        baseline: Some(baseline),
        target: Some(target),
        delta: Some(delta),
        evidence: accumulator.evidence,
        confidence,
        error_class: None,
        temp_file: None,
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

// =========================================================================
// Error & Temp File Findings
// =========================================================================

use regex::Regex;
use std::sync::OnceLock;

/// Parse a temp file log message, returning the size in bytes and optional statement.
pub fn parse_temp_file_message(msg: &str) -> Option<(u64, Option<String>)> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r#"temporary file:\s+path\s+"[^"]+",\s+size\s+(\d+)\s+bytes(?:,\s+(?:statement|query):\s+(.+))?"#).unwrap()
    });

    if let Some(caps) = re.captures(msg) {
        let size = caps.get(1)?.as_str().parse::<u64>().ok()?;
        let statement = caps.get(2).map(|m| m.as_str().to_string());
        Some((size, statement))
    } else {
        None
    }
}

/// Helper to search the events array for a nearby statement logged by the same PID.
pub fn find_nearby_statement(
    events: &[crate::NormalizedEvent],
    temp_event_idx: usize,
    pid: &str,
) -> Option<(String, Option<String>)> {
    // 1. Search backward
    for ev in events[..temp_event_idx].iter().rev() {
        if ev.session.process_id == pid {
            if let crate::EventKind::Statement(stmt) = &ev.kind {
                return Some((stmt.statement.clone(), ev.queryid.clone()));
            }
        }
    }
    // 2. Search forward
    for ev in events[(temp_event_idx + 1)..].iter() {
        if ev.session.process_id == pid {
            if let crate::EventKind::Statement(stmt) = &ev.kind {
                return Some((stmt.statement.clone(), ev.queryid.clone()));
            }
        }
    }
    None
}

/// Build ranked error-class findings from log events.
pub fn error_class_findings(events: &[crate::NormalizedEvent], limit: usize) -> FindingSet {
    let mut by_class: HashMap<String, ErrorClassAccumulator> = HashMap::new();

    for event in events {
        if let crate::EventKind::Error(error) = &event.kind {
            let key = error
                .sqlstate
                .clone()
                .unwrap_or_else(|| crate::events::normalize_error_message(&error.message));

            by_class
                .entry(key.clone())
                .or_insert_with(|| {
                    ErrorClassAccumulator::new(
                        error.sqlstate.clone(),
                        crate::events::normalize_error_message(&error.message),
                    )
                })
                .add_event(event);
        }
    }

    let mut accumulators: Vec<_> = by_class.into_values().collect();
    accumulators.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.sqlstate.cmp(&b.sqlstate))
            .then_with(|| a.normalized_message.cmp(&b.normalized_message))
    });

    let findings = accumulators
        .into_iter()
        .take(limit)
        .enumerate()
        .map(|(index, accumulator)| accumulator.into_finding(index + 1))
        .collect();

    FindingSet::new(findings)
}

struct ErrorClassAccumulator {
    sqlstate: Option<String>,
    normalized_message: String,
    count: u64,
    databases: HashMap<String, u64>,
    users: HashMap<String, u64>,
    app_names: HashMap<String, u64>,
    evidence: Vec<SourceReference>,
}

impl ErrorClassAccumulator {
    fn new(sqlstate: Option<String>, normalized_message: String) -> Self {
        Self {
            sqlstate,
            normalized_message,
            count: 0,
            databases: HashMap::new(),
            users: HashMap::new(),
            app_names: HashMap::new(),
            evidence: Vec::new(),
        }
    }

    fn add_event(&mut self, event: &crate::NormalizedEvent) {
        self.count += 1;
        if let Some(db) = &event.session.database {
            *self.databases.entry(db.clone()).or_insert(0) += 1;
        }
        if let Some(user) = &event.session.user {
            *self.users.entry(user.clone()).or_insert(0) += 1;
        }
        if let Some(app) = &event.session.application_name {
            *self.app_names.entry(app.clone()).or_insert(0) += 1;
        }
        if self.evidence.len() < 5 {
            self.evidence.push(event.source.clone());
        }
    }

    fn most_frequent(map: &HashMap<String, u64>) -> Option<String> {
        let mut entries: Vec<_> = map.iter().collect();
        entries.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
        entries.first().map(|e| e.0.clone())
    }

    fn into_finding(self, rank: usize) -> Finding {
        let database = Self::most_frequent(&self.databases);
        let user = Self::most_frequent(&self.users);
        let application_name = Self::most_frequent(&self.app_names);

        let title = if let Some(code) = &self.sqlstate {
            format!("Error Class {}: {}", code, self.normalized_message)
        } else {
            format!("Error Class: {}", self.normalized_message)
        };

        let reason = format!("Observed {} error events of this class.", self.count);

        let mut missing_attribution = Vec::new();
        if database.is_none() {
            missing_attribution.push(AttributionField::Database);
        }
        if user.is_none() {
            missing_attribution.push(AttributionField::User);
        }
        if application_name.is_none() {
            missing_attribution.push(AttributionField::ApplicationName);
        }

        Finding {
            schema_version: FINDING_SCHEMA_VERSION,
            finding_id: format!(
                "error_class:{}",
                self.sqlstate.as_deref().unwrap_or(&self.normalized_message)
            ),
            kind: FindingKind::ErrorClass,
            rank,
            title,
            reason,
            reason_codes: vec![ReasonCode::MeetsEligibilityThresholds],
            score: self.count as f64,
            query_family: None,
            metrics: FindingMetrics {
                execution_count: self.count,
                total_duration_ms: 0.0,
                avg_duration_ms: 0.0,
                max_duration_ms: 0.0,
                correlated_execution_count: self.count,
                uncorrelated_execution_count: 0,
            },
            baseline: None,
            target: None,
            delta: None,
            evidence: self.evidence,
            confidence: FindingConfidence::High,
            error_class: Some(ErrorClassFinding {
                sqlstate: self.sqlstate,
                normalized_error: self.normalized_message,
                database,
                user,
                application_name,
                error_count: self.count,
            }),
            temp_file: None,
        }
    }
}

/// Build ranked temp-file findings from log events.
pub fn temp_file_findings(events: &[crate::NormalizedEvent], limit: usize) -> FindingSet {
    let mut by_family: HashMap<String, TempFileAccumulator> = HashMap::new();

    for (index, event) in events.iter().enumerate() {
        if let Some((bytes, stmt_opt)) = parse_temp_file_message(event.message()) {
            let mut correlated_stmt = stmt_opt;
            let mut correlated_queryid = None;

            if correlated_stmt.is_none() {
                if let Some((nearby_sql, queryid_opt)) =
                    find_nearby_statement(events, index, &event.session.process_id)
                {
                    correlated_stmt = Some(nearby_sql);
                    correlated_queryid = queryid_opt;
                }
            }

            let (family_id, normalized_sql) = if let Some(stmt) = &correlated_stmt {
                let normalized = if let Ok(queries) = crate::Query::from_sql(stmt) {
                    queries
                        .iter()
                        .map(|q| q.normalized_query.clone())
                        .collect::<Vec<_>>()
                        .join(";")
                } else {
                    stmt.clone()
                };
                let identity = QueryFamilyIdentity::new(
                    normalized.clone(),
                    &event.session,
                    correlated_queryid,
                );
                (Some(identity.family_id), Some(normalized))
            } else {
                (None, None)
            };

            let key = family_id
                .clone()
                .unwrap_or_else(|| "unknown_statement".to_string());

            by_family
                .entry(key.clone())
                .or_insert_with(|| TempFileAccumulator::new(family_id, normalized_sql))
                .add_event(event, bytes);
        }
    }

    let mut accumulators: Vec<_> = by_family.into_values().collect();
    accumulators.sort_by(|a, b| {
        b.total_bytes
            .cmp(&a.total_bytes)
            .then_with(|| b.count.cmp(&a.count))
            .then_with(|| a.query_family_id.cmp(&b.query_family_id))
    });

    let findings = accumulators
        .into_iter()
        .take(limit)
        .enumerate()
        .map(|(index, accumulator)| accumulator.into_finding(index + 1))
        .collect();

    FindingSet::new(findings)
}

struct TempFileAccumulator {
    query_family_id: Option<String>,
    normalized_sql: Option<String>,
    count: u64,
    total_bytes: u64,
    largest_observed_bytes: u64,
    databases: HashMap<String, u64>,
    users: HashMap<String, u64>,
    app_names: HashMap<String, u64>,
    evidence: Vec<SourceReference>,
}

impl TempFileAccumulator {
    fn new(query_family_id: Option<String>, normalized_sql: Option<String>) -> Self {
        Self {
            query_family_id,
            normalized_sql,
            count: 0,
            total_bytes: 0,
            largest_observed_bytes: 0,
            databases: HashMap::new(),
            users: HashMap::new(),
            app_names: HashMap::new(),
            evidence: Vec::new(),
        }
    }

    fn add_event(&mut self, event: &crate::NormalizedEvent, bytes: u64) {
        self.count += 1;
        self.total_bytes += bytes;
        self.largest_observed_bytes = self.largest_observed_bytes.max(bytes);

        if let Some(db) = &event.session.database {
            *self.databases.entry(db.clone()).or_insert(0) += 1;
        }
        if let Some(user) = &event.session.user {
            *self.users.entry(user.clone()).or_insert(0) += 1;
        }
        if let Some(app) = &event.session.application_name {
            *self.app_names.entry(app.clone()).or_insert(0) += 1;
        }
        if self.evidence.len() < 5 {
            self.evidence.push(event.source.clone());
        }
    }

    fn most_frequent(map: &HashMap<String, u64>) -> Option<String> {
        let mut entries: Vec<_> = map.iter().collect();
        entries.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
        entries.first().map(|e| e.0.clone())
    }

    fn into_finding(self, rank: usize) -> Finding {
        let database = Self::most_frequent(&self.databases);
        let user = Self::most_frequent(&self.users);
        let application_name = Self::most_frequent(&self.app_names);

        let title = if let Some(qfid) = &self.query_family_id {
            format!("Temporary Files: {}", qfid)
        } else {
            "Temporary Files: Unknown Statement".to_string()
        };

        let reason = format!(
            "Observed {} temporary file events writing {} bytes in total. Largest file was {} bytes.",
            self.count, self.total_bytes, self.largest_observed_bytes
        );

        let mut missing_attribution = Vec::new();
        if database.is_none() {
            missing_attribution.push(AttributionField::Database);
        }
        if user.is_none() {
            missing_attribution.push(AttributionField::User);
        }
        if application_name.is_none() {
            missing_attribution.push(AttributionField::ApplicationName);
        }

        Finding {
            schema_version: FINDING_SCHEMA_VERSION,
            finding_id: format!(
                "temp_file:{}",
                self.query_family_id
                    .as_deref()
                    .unwrap_or("unknown_statement")
            ),
            kind: FindingKind::TempFile,
            rank,
            title,
            reason,
            reason_codes: vec![ReasonCode::HighTotalDuration],
            score: self.total_bytes as f64,
            query_family: None,
            metrics: FindingMetrics {
                execution_count: self.count,
                total_duration_ms: self.total_bytes as f64,
                avg_duration_ms: (self.total_bytes as f64) / (self.count as f64),
                max_duration_ms: self.largest_observed_bytes as f64,
                correlated_execution_count: self.count,
                uncorrelated_execution_count: 0,
            },
            baseline: None,
            target: None,
            delta: None,
            evidence: self.evidence,
            confidence: if self.query_family_id.is_some() {
                FindingConfidence::High
            } else {
                FindingConfidence::Low
            },
            error_class: None,
            temp_file: Some(TempFileFinding {
                query_family_id: self.query_family_id,
                normalized_sql: self.normalized_sql,
                database,
                user,
                application_name,
                largest_observed_bytes: self.largest_observed_bytes,
                temp_file_count: self.count,
                total_bytes: self.total_bytes,
            }),
        }
    }
}

pub fn errors_report(
    findings: FindingSet,
    entries: &[LogEntry],
    source_kind: EventSourceKind,
) -> PgTriageReport<FindingsPayload> {
    PgTriageReport {
        schema_version: PG_TRIAGE_SCHEMA_VERSION,
        workflow: ActionKind::Errors,
        operating_mode: OperatingMode::LogBackedOnly,
        limitations: Vec::new(),
        verdict: Some(Verdict::Unknown),
        verdict_reasons: vec!["live_state_verdict_not_evaluated".to_string()],
        allowed_actions: None,
        blocked_actions: None,
        analysis_window: analysis_window(entries),
        source_summary: Some(SourceSummary {
            kind: SourceSummaryKind::from(source_kind),
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

pub fn temp_files_report(
    findings: FindingSet,
    entries: &[LogEntry],
    source_kind: EventSourceKind,
    has_uncorrelated: bool,
) -> PgTriageReport<FindingsPayload> {
    let mut limitations = Vec::new();
    if has_uncorrelated {
        limitations.push(
            "Some temporary file events could not be correlated with a SQL statement.".to_string(),
        );
    }

    PgTriageReport {
        schema_version: PG_TRIAGE_SCHEMA_VERSION,
        workflow: ActionKind::TempFiles,
        operating_mode: OperatingMode::LogBackedOnly,
        limitations,
        verdict: Some(Verdict::Unknown),
        verdict_reasons: vec!["live_state_verdict_not_evaluated".to_string()],
        allowed_actions: None,
        blocked_actions: None,
        analysis_window: analysis_window(entries),
        source_summary: Some(SourceSummary {
            kind: SourceSummaryKind::from(source_kind),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CorrelationConfidence, EventSourceKind, Query, QueryExecution, QueryFamilyIdentity,
        SessionIdentity, SourceReference,
    };
    use chrono::{TimeZone, Utc};

    fn execution(sql: &str, duration_ms: Option<f64>, record_index: usize) -> QueryExecution {
        let session = SessionIdentity {
            process_id: "12345".to_string(),
            user: Some("app".to_string()),
            database: Some("appdb".to_string()),
            client_host: None,
            application_name: Some("api".to_string()),
        };
        let queries = Query::from_sql(sql).unwrap();
        let normalized_sql = queries[0].normalized_query.clone();
        let query_family = QueryFamilyIdentity::new(normalized_sql, &session, None);

        QueryExecution {
            execution_id: format!("stderr:{record_index}"),
            timestamp: Utc.with_ymd_and_hms(2024, 8, 15, 10, 30, 0).unwrap(),
            session,
            statement: sql.to_string(),
            queries,
            query_family,
            duration_ms,
            evidence: vec![SourceReference {
                source_kind: EventSourceKind::Stderr,
                record_index,
            }],
            confidence: if duration_ms.is_some() {
                CorrelationConfidence::Exact
            } else {
                CorrelationConfidence::StatementOnly
            },
        }
    }

    #[test]
    fn ranks_query_family_findings_by_total_duration() {
        let executions = vec![
            execution("SELECT * FROM users WHERE id = 1", Some(50.0), 0),
            execution("SELECT * FROM users WHERE id = 2", Some(75.0), 1),
            execution("SELECT * FROM orders WHERE id = 1", Some(250.0), 2),
        ];

        let findings = query_family_findings(&executions, 10);

        assert_eq!(findings.schema_version, 1);
        assert_eq!(findings.findings.len(), 2);
        assert_eq!(findings.findings[0].rank, 1);
        assert_eq!(findings.findings[0].metrics.total_duration_ms, 250.0);
        assert_eq!(findings.findings[1].metrics.total_duration_ms, 125.0);
    }

    #[test]
    fn includes_evidence_and_correlation_reason_codes() {
        let executions = vec![
            execution("SELECT * FROM users WHERE id = 1", Some(50.0), 0),
            execution("SELECT * FROM users WHERE id = 2", None, 1),
        ];

        let findings = query_family_findings(&executions, 10);
        let finding = &findings.findings[0];

        assert_eq!(finding.schema_version, 1);
        assert_eq!(finding.kind, FindingKind::QueryFamily);
        assert_eq!(finding.confidence, FindingConfidence::Medium);
        assert_eq!(finding.evidence.len(), 2);
        assert!(finding
            .reason_codes
            .contains(&ReasonCode::CorrelatedDuration));
        assert!(finding
            .reason_codes
            .contains(&ReasonCode::PartialCorrelation));
        assert_eq!(finding.metrics.execution_count, 2);
        assert_eq!(finding.metrics.correlated_execution_count, 1);
        assert_eq!(finding.metrics.uncorrelated_execution_count, 1);
    }

    #[test]
    fn represents_missing_attribution_explicitly() {
        let session = SessionIdentity {
            process_id: "12345".to_string(),
            user: None,
            database: None,
            client_host: None,
            application_name: None,
        };
        let queries = Query::from_sql("SELECT * FROM users WHERE id = 1").unwrap();
        let normalized_sql = queries[0].normalized_query.clone();
        let query_family = QueryFamilyIdentity::new(normalized_sql, &session, None);

        let mut findings = query_family_findings(
            &[QueryExecution {
                execution_id: "stderr:0".to_string(),
                timestamp: Utc.with_ymd_and_hms(2024, 8, 15, 10, 30, 0).unwrap(),
                session,
                statement: "SELECT * FROM users WHERE id = 1".to_string(),
                queries,
                query_family,
                duration_ms: Some(50.0),
                evidence: vec![SourceReference {
                    source_kind: EventSourceKind::Stderr,
                    record_index: 0,
                }],
                confidence: CorrelationConfidence::Exact,
            }],
            10,
        );
        let finding = findings.findings.remove(0);

        let query_family = finding.query_family.unwrap();
        assert_eq!(query_family.database, None);
        assert_eq!(query_family.user, None);
        assert_eq!(query_family.application_name, None);
        assert_eq!(
            query_family.missing_attribution,
            vec![
                AttributionField::ApplicationName,
                AttributionField::Database,
                AttributionField::User,
            ]
        );
    }

    #[test]
    fn slow_query_diff_flags_query_absent_in_baseline() {
        let baseline = vec![execution("SELECT * FROM users WHERE id = 1", Some(25.0), 0)];
        let target = vec![execution(
            "SELECT * FROM orders WHERE id = 1",
            Some(200.0),
            1,
        )];

        let findings = slow_query_diff_findings(
            &baseline,
            &target,
            SlowQueryDiffOptions {
                limit: 10,
                min_target_count: 1,
                min_target_total_ms: 0.0,
                min_p95_delta_ms: 0.0,
            },
        );

        assert_eq!(findings.findings.len(), 1);
        let finding = &findings.findings[0];
        assert_eq!(finding.kind, FindingKind::SlowQueryRegression);
        assert!(finding.reason_codes.contains(&ReasonCode::AbsentInBaseline));
        assert_eq!(finding.baseline.unwrap().execution_count, 0);
        assert_eq!(finding.target.unwrap().total_duration_ms, 200.0);
        assert_eq!(finding.delta.unwrap().total_duration_ms, 200.0);
    }

    #[test]
    fn slow_query_diff_flags_p95_regression() {
        let baseline = vec![
            execution("SELECT * FROM users WHERE id = 1", Some(20.0), 0),
            execution("SELECT * FROM users WHERE id = 2", Some(30.0), 1),
        ];
        let target = vec![
            execution("SELECT * FROM users WHERE id = 3", Some(100.0), 2),
            execution("SELECT * FROM users WHERE id = 4", Some(150.0), 3),
        ];

        let findings = slow_query_diff_findings(
            &baseline,
            &target,
            SlowQueryDiffOptions {
                limit: 10,
                min_target_count: 1,
                min_target_total_ms: 0.0,
                min_p95_delta_ms: 50.0,
            },
        );

        assert_eq!(findings.findings.len(), 1);
        let finding = &findings.findings[0];
        assert!(finding.reason_codes.contains(&ReasonCode::P95Regressed));
        assert!(finding
            .reason_codes
            .contains(&ReasonCode::RuntimeContributionIncreased));
        assert_eq!(finding.baseline.unwrap().p95_duration_ms, 30.0);
        assert_eq!(finding.target.unwrap().p95_duration_ms, 150.0);
        assert_eq!(finding.delta.unwrap().p95_duration_ms, 120.0);
    }

    #[test]
    fn slow_query_diff_applies_target_eligibility_thresholds() {
        let baseline = Vec::new();
        let target = vec![execution("SELECT * FROM users WHERE id = 1", Some(20.0), 0)];

        let findings = slow_query_diff_findings(
            &baseline,
            &target,
            SlowQueryDiffOptions {
                limit: 10,
                min_target_count: 2,
                min_target_total_ms: 100.0,
                min_p95_delta_ms: 0.0,
            },
        );

        assert!(findings.findings.is_empty());
    }
}
