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
    pub evidence: Vec<SourceReference>,
    pub confidence: FindingConfidence,
    pub next_sql: Vec<String>,
}

/// Finding family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingKind {
    QueryFamily,
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

        let next_sql = self
            .identity
            .queryid
            .as_ref()
            .map(|queryid| {
                vec![format!(
                    "select * from pg_stat_statements where queryid = {};",
                    queryid
                )]
            })
            .unwrap_or_default();

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
}
