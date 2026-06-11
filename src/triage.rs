use crate::{EventSourceKind, Finding, FindingSet, LogEntry};
use serde::{Deserialize, Serialize};

pub const PG_TRIAGE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatingMode {
    LogBacked,
    LiveOnly,
    Unready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowId {
    AgentInstall,
    Inspect,
    RunningQueries,
    TopQueryFamilies,
    Errors,
    TempFiles,
    SuggestSql,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Clear,
    Busy,
    Saturated,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLabel {
    Safe,
    Bounded,
    Expensive,
    RequiresHumanApproval,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionClass {
    SystemCatalogReads,
    BoundedActivityQueries,
    StatsViewReads,
    TextPatternStatsSearch,
    ExplainWithoutAnalyze,
    LargeUnboundedSelects,
    ExplainAnalyze,
    WriteOrAdminAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionStatus {
    Allowed,
    BlockedByVerdict,
    BlockedByConfig,
    BlockedByPolicy,
    OmittedNotEnoughContext,
    OmittedUnsupportedTarget,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Passed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceSummaryKind {
    LocalStderr,
    AwsRds,
    Csvlog,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalysisWindow {
    pub since: String,
    pub until: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSummary {
    pub kind: SourceSummaryKind,
    pub entries_scanned: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingsPayload {
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PgTriageReport<T> {
    pub schema_version: u32,
    pub workflow: WorkflowId,
    pub operating_mode: OperatingMode,
    pub limitations: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verdict: Option<Verdict>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub verdict_reasons: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_actions: Option<Vec<ActionClass>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocked_actions: Option<Vec<ActionClass>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub analysis_window: Option<AnalysisWindow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_summary: Option<SourceSummary>,
    pub payload: T,
}

pub fn top_query_families_report(
    findings: FindingSet,
    entries: &[LogEntry],
    source_kind: EventSourceKind,
) -> PgTriageReport<FindingsPayload> {
    PgTriageReport {
        schema_version: PG_TRIAGE_SCHEMA_VERSION,
        workflow: WorkflowId::TopQueryFamilies,
        operating_mode: OperatingMode::LogBacked,
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
        payload: FindingsPayload {
            findings: findings.findings,
        },
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
        assert_eq!(value["operating_mode"], "log_backed");
        assert_eq!(value["verdict"], "unknown");
        assert_eq!(value["source_summary"]["kind"], "local_stderr");
        assert_eq!(value["source_summary"]["entries_scanned"], 2);
        assert_eq!(
            value["payload"]["findings"][0]["finding_id"],
            "query_family:demo"
        );
    }
}
