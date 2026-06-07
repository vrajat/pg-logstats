//! Structured findings for investigation-oriented output.

use crate::{CorrelationConfidence, QueryExecution, QueryFamilyIdentity, SourceReference};
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
    pub next_sql: Vec<String>,
}

/// Finding family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingKind {
    QueryFamily,
    SlowQueryRegression,
    ErrorClass,
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
    pub normalized_sql: String,
    pub queryid: Option<String>,
    pub database: Option<String>,
    pub user: Option<String>,
    pub application_name: Option<String>,
}

impl From<&QueryFamilyIdentity> for QueryFamilyFinding {
    fn from(identity: &QueryFamilyIdentity) -> Self {
        Self {
            query_family_id: identity.family_id.clone(),
            normalized_sql: identity.normalized_sql.clone(),
            queryid: identity.queryid.clone(),
            database: identity.database.clone(),
            user: identity.user.clone(),
            application_name: identity.application_name.clone(),
        }
    }
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

        let next_sql = suggest_sql_for_query_family(&self.identity);

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
            next_sql,
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
        next_sql: suggest_sql_for_query_family(&accumulator.identity),
    }
}

/// Build follow-up SQL suggestions for a query-family finding.
pub fn suggest_sql_for_query_family(identity: &QueryFamilyIdentity) -> Vec<String> {
    let mut statements = Vec::new();

    if let Some(queryid) = &identity.queryid {
        statements.push(format!(
            "select queryid, calls, total_exec_time, mean_exec_time, rows, query \
from pg_stat_statements where queryid = {};",
            queryid
        ));
    } else {
        statements.push(format!(
            "select queryid, calls, total_exec_time, mean_exec_time, rows, query \
from pg_stat_statements where query ilike '%{}%' order by total_exec_time desc limit 20;",
            escape_like_literal(&identity.normalized_sql)
        ));
    }

    let mut activity_filters = Vec::new();
    if let Some(database) = &identity.database {
        activity_filters.push(format!("datname = '{}'", escape_sql_literal(database)));
    }
    if let Some(user) = &identity.user {
        activity_filters.push(format!("usename = '{}'", escape_sql_literal(user)));
    }
    if let Some(application_name) = &identity.application_name {
        activity_filters.push(format!(
            "application_name = '{}'",
            escape_sql_literal(application_name)
        ));
    }

    let where_clause = if activity_filters.is_empty() {
        "state <> 'idle'".to_string()
    } else {
        activity_filters.join(" and ")
    };

    statements.push(format!(
        "select pid, usename, datname, application_name, state, wait_event_type, \
wait_event, query_start, query from pg_stat_activity where {} \
order by query_start desc nulls last limit 20;",
        where_clause
    ));

    statements
}

fn escape_sql_literal(value: &str) -> String {
    value.replace('\'', "''")
}

fn escape_like_literal(value: &str) -> String {
    escape_sql_literal(value)
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
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
        assert_eq!(finding.next_sql.len(), 2);
        assert!(finding.next_sql[0].contains("pg_stat_statements"));
        assert!(finding.next_sql[1].contains("pg_stat_activity"));
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
        assert_eq!(finding.next_sql.len(), 2);
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

    #[test]
    fn suggest_sql_escapes_identity_fields() {
        let session = SessionIdentity {
            process_id: "12345".to_string(),
            user: Some("app'user".to_string()),
            database: Some("app_db".to_string()),
            client_host: None,
            application_name: Some("api%worker".to_string()),
        };
        let identity = QueryFamilyIdentity::new(
            "select * from orders where note = 'abc_%'".to_string(),
            &session,
            None,
        );

        let sql = suggest_sql_for_query_family(&identity);

        assert_eq!(sql.len(), 2);
        assert!(sql[0].contains("pg_stat_statements"));
        assert!(sql[0].contains("abc\\_\\%"));
        assert!(sql[1].contains("usename = 'app''user'"));
        assert!(sql[1].contains("application_name = 'api%worker'"));
    }
}
