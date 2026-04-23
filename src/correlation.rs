//! Correlation layer for normalized PostgreSQL events.
//!
//! The first correlation slice pairs statement events with following duration
//! events from the same backend process. This is intentionally modeled as a
//! strategy so structured log implementations can later use stronger keys such
//! as session ID and per-session line number.

use crate::{EventKind, NormalizedEvent, Query, SessionIdentity, SourceReference, StatementEvent};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A deterministic grouping key for related executions of the same query shape.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct QueryFamilyIdentity {
    pub family_id: String,
    pub normalized_sql: String,
    pub database: Option<String>,
    pub user: Option<String>,
    pub application_name: Option<String>,
    pub queryid: Option<String>,
}

impl QueryFamilyIdentity {
    pub fn new(normalized_sql: String, session: &SessionIdentity, queryid: Option<String>) -> Self {
        let family_id = format!(
            "queryid={}|db={}|user={}|app={}|sql={}",
            queryid.as_deref().unwrap_or(""),
            session.database.as_deref().unwrap_or(""),
            session.user.as_deref().unwrap_or(""),
            session.application_name.as_deref().unwrap_or(""),
            normalized_sql
        );

        Self {
            family_id,
            normalized_sql,
            database: session.database.clone(),
            user: session.user.clone(),
            application_name: session.application_name.clone(),
            queryid,
        }
    }
}

/// How confidently the execution was reconstructed from raw events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CorrelationConfidence {
    Exact,
    StatementOnly,
}

/// A correlated query execution suitable for analytics and findings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryExecution {
    pub execution_id: String,
    pub timestamp: DateTime<Utc>,
    pub session: SessionIdentity,
    pub statement: String,
    pub queries: Vec<Query>,
    pub query_family: QueryFamilyIdentity,
    pub duration_ms: Option<f64>,
    pub evidence: Vec<SourceReference>,
    pub confidence: CorrelationConfidence,
}

#[derive(Debug, Clone)]
struct PendingStatement {
    event_id: String,
    timestamp: DateTime<Utc>,
    source: SourceReference,
    session: SessionIdentity,
    queryid: Option<String>,
    statement: StatementEvent,
}

/// Strategy interface for reconstructing higher-level query executions.
pub trait Correlator {
    fn correlate(&self, events: &[NormalizedEvent]) -> Vec<QueryExecution>;
}

/// Correlates statement and duration events by process ID and stream order.
///
/// This fits current stderr parsing and remains a fallback for structured logs
/// that do not provide session line numbers. Future csvlog/jsonlog/RDS/GCP
/// implementations should prefer session-aware keys where available.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProcessOrderCorrelator;

impl Correlator for ProcessOrderCorrelator {
    fn correlate(&self, events: &[NormalizedEvent]) -> Vec<QueryExecution> {
        correlate_by_process_order(events)
    }
}

/// Pair statement and duration events into query executions using the default
/// process-order strategy.
pub fn correlate_query_executions(events: &[NormalizedEvent]) -> Vec<QueryExecution> {
    ProcessOrderCorrelator.correlate(events)
}

fn correlate_by_process_order(events: &[NormalizedEvent]) -> Vec<QueryExecution> {
    let mut executions = Vec::new();
    let mut pending_by_process: HashMap<String, PendingStatement> = HashMap::new();

    for event in events {
        match &event.kind {
            EventKind::Statement(statement) => {
                if let Some(previous) = pending_by_process.remove(&event.session.process_id) {
                    executions.push(execution_from_pending(
                        previous,
                        None,
                        None,
                        CorrelationConfidence::StatementOnly,
                    ));
                }

                if let Some(duration_ms) = statement.duration_ms {
                    executions.push(execution_from_statement_event(
                        event,
                        statement,
                        Some(duration_ms),
                        vec![event.source.clone()],
                        CorrelationConfidence::Exact,
                    ));
                } else {
                    pending_by_process.insert(
                        event.session.process_id.clone(),
                        PendingStatement {
                            event_id: event.event_id.clone(),
                            timestamp: event.timestamp,
                            source: event.source.clone(),
                            session: event.session.clone(),
                            queryid: event.queryid.clone(),
                            statement: statement.clone(),
                        },
                    );
                }
            }
            EventKind::Duration(duration) => {
                if let Some(pending) = pending_by_process.remove(&event.session.process_id) {
                    if event.timestamp >= pending.timestamp {
                        executions.push(execution_from_pending(
                            pending,
                            Some(duration.duration_ms),
                            Some(event.source.clone()),
                            CorrelationConfidence::Exact,
                        ));
                    } else {
                        pending_by_process.insert(event.session.process_id.clone(), pending);
                    }
                }
            }
            _ => {}
        }
    }

    let mut remaining: Vec<_> = pending_by_process.into_values().collect();
    remaining.sort_by_key(|pending| pending.timestamp);
    for pending in remaining {
        executions.push(execution_from_pending(
            pending,
            None,
            None,
            CorrelationConfidence::StatementOnly,
        ));
    }

    executions.sort_by_key(|execution| execution.timestamp);
    executions
}

fn execution_from_pending(
    pending: PendingStatement,
    duration_ms: Option<f64>,
    duration_source: Option<SourceReference>,
    confidence: CorrelationConfidence,
) -> QueryExecution {
    let mut evidence = vec![pending.source];
    if let Some(duration_source) = duration_source {
        evidence.push(duration_source);
    }

    let normalized_sql = normalized_sql(&pending.statement);
    let query_family = QueryFamilyIdentity::new(normalized_sql, &pending.session, pending.queryid);

    QueryExecution {
        execution_id: pending.event_id,
        timestamp: pending.timestamp,
        session: pending.session,
        statement: pending.statement.statement,
        queries: pending.statement.queries,
        query_family,
        duration_ms,
        evidence,
        confidence,
    }
}

fn execution_from_statement_event(
    event: &NormalizedEvent,
    statement: &StatementEvent,
    duration_ms: Option<f64>,
    evidence: Vec<SourceReference>,
    confidence: CorrelationConfidence,
) -> QueryExecution {
    let normalized_sql = normalized_sql(statement);
    let query_family =
        QueryFamilyIdentity::new(normalized_sql, &event.session, event.queryid.clone());

    QueryExecution {
        execution_id: event.event_id.clone(),
        timestamp: event.timestamp,
        session: event.session.clone(),
        statement: statement.statement.clone(),
        queries: statement.queries.clone(),
        query_family,
        duration_ms,
        evidence,
        confidence,
    }
}

fn normalized_sql(statement: &StatementEvent) -> String {
    if statement.queries.is_empty() {
        statement.statement.clone()
    } else {
        statement
            .queries
            .iter()
            .map(|query| query.normalized_query.clone())
            .collect::<Vec<_>>()
            .join(";")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DurationEvent, EventKind, EventSourceKind, NormalizedEvent, Query, SourceReference,
    };
    use chrono::{Duration, TimeZone};

    fn session(process_id: &str, database: &str) -> SessionIdentity {
        SessionIdentity {
            process_id: process_id.to_string(),
            user: Some("postgres".to_string()),
            database: Some(database.to_string()),
            client_host: None,
            application_name: Some("psql".to_string()),
        }
    }

    fn statement_event(index: usize, process_id: &str, sql: &str) -> NormalizedEvent {
        NormalizedEvent {
            event_id: format!("stderr:{index}"),
            timestamp: Utc.with_ymd_and_hms(2024, 8, 15, 10, 30, 0).unwrap()
                + Duration::milliseconds(index as i64),
            source: SourceReference {
                source_kind: EventSourceKind::Stderr,
                record_index: index,
            },
            session: session(process_id, "testdb"),
            queryid: None,
            kind: EventKind::Statement(StatementEvent {
                statement: sql.to_string(),
                queries: Query::from_sql(sql).unwrap(),
                duration_ms: None,
            }),
        }
    }

    fn duration_event(index: usize, process_id: &str, duration_ms: f64) -> NormalizedEvent {
        NormalizedEvent {
            event_id: format!("stderr:{index}"),
            timestamp: Utc.with_ymd_and_hms(2024, 8, 15, 10, 30, 0).unwrap()
                + Duration::milliseconds(index as i64),
            source: SourceReference {
                source_kind: EventSourceKind::Stderr,
                record_index: index,
            },
            session: session(process_id, "testdb"),
            queryid: None,
            kind: EventKind::Duration(DurationEvent { duration_ms }),
        }
    }

    #[test]
    fn pairs_statement_with_following_duration_on_same_process() {
        let events = vec![
            statement_event(0, "12345", "SELECT * FROM users WHERE id = 1"),
            duration_event(1, "12345", 42.5),
        ];

        let executions = ProcessOrderCorrelator.correlate(&events);

        assert_eq!(executions.len(), 1);
        assert_eq!(executions[0].duration_ms, Some(42.5));
        assert_eq!(executions[0].confidence, CorrelationConfidence::Exact);
        assert_eq!(executions[0].evidence.len(), 2);
        assert_eq!(executions[0].evidence[0].record_index, 0);
        assert_eq!(executions[0].evidence[1].record_index, 1);
    }

    #[test]
    fn default_correlation_function_uses_process_order_strategy() {
        let events = vec![
            statement_event(0, "12345", "SELECT * FROM users WHERE id = 1"),
            duration_event(1, "12345", 42.5),
        ];

        let via_function = correlate_query_executions(&events);
        let via_strategy = ProcessOrderCorrelator.correlate(&events);

        assert_eq!(via_function.len(), via_strategy.len());
        assert_eq!(via_function[0].duration_ms, via_strategy[0].duration_ms);
        assert_eq!(via_function[0].evidence, via_strategy[0].evidence);
    }

    #[test]
    fn does_not_pair_duration_from_another_process() {
        let events = vec![
            statement_event(0, "11111", "SELECT * FROM users WHERE id = 1"),
            duration_event(1, "22222", 42.5),
        ];

        let executions = correlate_query_executions(&events);

        assert_eq!(executions.len(), 1);
        assert_eq!(executions[0].duration_ms, None);
        assert_eq!(
            executions[0].confidence,
            CorrelationConfidence::StatementOnly
        );
        assert_eq!(executions[0].evidence.len(), 1);
    }

    #[test]
    fn flushes_previous_pending_statement_when_same_process_starts_new_statement() {
        let events = vec![
            statement_event(0, "12345", "SELECT * FROM users WHERE id = 1"),
            statement_event(1, "12345", "SELECT * FROM posts WHERE id = 2"),
            duration_event(2, "12345", 12.0),
        ];

        let executions = correlate_query_executions(&events);

        assert_eq!(executions.len(), 2);
        assert_eq!(executions[0].duration_ms, None);
        assert_eq!(
            executions[0].confidence,
            CorrelationConfidence::StatementOnly
        );
        assert_eq!(executions[1].duration_ms, Some(12.0));
        assert_eq!(executions[1].confidence, CorrelationConfidence::Exact);
    }

    #[test]
    fn query_family_identity_includes_normalized_sql_and_metadata() {
        let mut event = statement_event(0, "12345", "SELECT * FROM users WHERE id = 1");
        event.session.database = Some("analytics".to_string());
        event.session.user = Some("reporter".to_string());
        event.session.application_name = Some("dashboard".to_string());
        let events = vec![event, duration_event(1, "12345", 5.0)];

        let executions = correlate_query_executions(&events);
        let family = &executions[0].query_family;

        assert_eq!(family.normalized_sql, "SELECT * FROM users WHERE id = ?");
        assert_eq!(family.database.as_deref(), Some("analytics"));
        assert_eq!(family.user.as_deref(), Some("reporter"));
        assert_eq!(family.application_name.as_deref(), Some("dashboard"));
        assert_eq!(
            family.family_id,
            "queryid=|db=analytics|user=reporter|app=dashboard|sql=SELECT * FROM users WHERE id = ?"
        );
    }
}
