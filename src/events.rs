//! Normalized event model for PostgreSQL log analysis.
//!
//! This layer sits above raw parser output so workflows and analytics do not
//! depend directly on the legacy `LogEntry` structure.

use crate::{LogEntry, LogLevel, Query};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The parser/source format that produced an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventSourceKind {
    Stderr,
    Csvlog,
    Jsonlog,
}

/// Stable pointer back to the raw source record that produced an event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceReference {
    pub source_kind: EventSourceKind,
    pub record_index: usize,
}

/// Identity metadata carried across related PostgreSQL events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionIdentity {
    pub process_id: String,
    pub user: Option<String>,
    pub database: Option<String>,
    pub client_host: Option<String>,
    pub application_name: Option<String>,
}

/// Structured statement payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatementEvent {
    pub statement: String,
    pub queries: Vec<Query>,
    pub duration_ms: Option<f64>,
}

/// Structured duration payload.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DurationEvent {
    pub duration_ms: f64,
}

/// Structured error payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorEvent {
    pub message: String,
    pub sqlstate: Option<String>,
}

/// Normalized event kinds for investigation workflows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventKind {
    Statement(StatementEvent),
    Duration(DurationEvent),
    Error(ErrorEvent),
    Log { level: LogLevel, message: String },
}

/// Normalized PostgreSQL event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalizedEvent {
    pub event_id: String,
    pub timestamp: DateTime<Utc>,
    pub source: SourceReference,
    pub session: SessionIdentity,
    pub queryid: Option<String>,
    pub kind: EventKind,
}

impl NormalizedEvent {
    pub fn from_log_entry(
        entry: &LogEntry,
        source_kind: EventSourceKind,
        record_index: usize,
    ) -> Self {
        let source = SourceReference {
            source_kind,
            record_index,
        };

        let session = SessionIdentity {
            process_id: entry.process_id.clone(),
            user: entry.user.clone(),
            database: entry.database.clone(),
            client_host: entry.client_host.clone(),
            application_name: entry.application_name.clone(),
        };

        let kind = if entry.is_query() {
            EventKind::Statement(StatementEvent {
                statement: entry
                    .message
                    .strip_prefix("statement: ")
                    .unwrap_or(&entry.message)
                    .to_string(),
                queries: entry.queries.clone().unwrap_or_default(),
                duration_ms: entry.duration,
            })
        } else if entry.is_duration() {
            EventKind::Duration(DurationEvent {
                duration_ms: entry.duration.unwrap_or(0.0),
            })
        } else if entry.is_error() {
            EventKind::Error(ErrorEvent {
                message: entry.message.clone(),
                sqlstate: None,
            })
        } else {
            EventKind::Log {
                level: entry.message_type.clone(),
                message: entry.message.clone(),
            }
        };

        Self {
            event_id: format!(
                "{}:{}",
                match source_kind {
                    EventSourceKind::Stderr => "stderr",
                    EventSourceKind::Csvlog => "csvlog",
                    EventSourceKind::Jsonlog => "jsonlog",
                },
                record_index
            ),
            timestamp: entry.timestamp,
            source,
            session,
            queryid: None,
            kind,
        }
    }

    pub fn is_query(&self) -> bool {
        matches!(self.kind, EventKind::Statement(_))
    }

    pub fn is_error(&self) -> bool {
        matches!(self.kind, EventKind::Error(_))
    }

    pub fn duration_ms(&self) -> Option<f64> {
        match &self.kind {
            EventKind::Statement(statement) => statement.duration_ms,
            EventKind::Duration(duration) => Some(duration.duration_ms),
            _ => None,
        }
    }

    pub fn queries(&self) -> Option<&[Query]> {
        match &self.kind {
            EventKind::Statement(statement) => Some(&statement.queries),
            _ => None,
        }
    }

    pub fn normalized_query(&self) -> Option<String> {
        let queries = self.queries()?;
        if queries.is_empty() {
            return None;
        }

        Some(
            queries
                .iter()
                .map(|query| query.normalized_query.clone())
                .collect::<Vec<_>>()
                .join(";"),
        )
    }

    pub fn message(&self) -> &str {
        match &self.kind {
            EventKind::Statement(statement) => &statement.statement,
            EventKind::Duration(_) => "",
            EventKind::Error(error) => &error.message,
            EventKind::Log { message, .. } => message,
        }
    }
}

pub fn normalize_log_entries(
    entries: &[LogEntry],
    source_kind: EventSourceKind,
) -> Vec<NormalizedEvent> {
    entries
        .iter()
        .enumerate()
        .map(|(record_index, entry)| {
            NormalizedEvent::from_log_entry(entry, source_kind, record_index)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LogEntry, LogLevel};
    use chrono::TimeZone;

    fn entry(
        message_type: LogLevel,
        message: &str,
        duration: Option<f64>,
        queries: Option<Vec<Query>>,
    ) -> LogEntry {
        LogEntry {
            timestamp: Utc.with_ymd_and_hms(2024, 8, 15, 10, 30, 0).unwrap(),
            process_id: "12345".to_string(),
            user: Some("postgres".to_string()),
            database: Some("testdb".to_string()),
            client_host: Some("10.0.0.10".to_string()),
            application_name: Some("psql".to_string()),
            message_type,
            message: message.to_string(),
            queries,
            duration,
        }
    }

    #[test]
    fn converts_statement_entries_into_normalized_events() {
        let entry = entry(
            LogLevel::Statement,
            "statement: SELECT * FROM users WHERE id = 1",
            Some(42.0),
            crate::Query::from_sql("SELECT * FROM users WHERE id = 1").ok(),
        );

        let event = NormalizedEvent::from_log_entry(&entry, EventSourceKind::Stderr, 7);

        assert_eq!(event.event_id, "stderr:7");
        assert_eq!(event.source.record_index, 7);
        assert_eq!(event.source.source_kind, EventSourceKind::Stderr);
        assert_eq!(event.session.process_id, "12345");
        assert_eq!(event.session.user.as_deref(), Some("postgres"));
        assert_eq!(event.session.database.as_deref(), Some("testdb"));
        assert_eq!(event.session.client_host.as_deref(), Some("10.0.0.10"));
        assert_eq!(event.session.application_name.as_deref(), Some("psql"));
        assert!(event.is_query());
        assert_eq!(event.duration_ms(), Some(42.0));
        assert_eq!(event.message(), "SELECT * FROM users WHERE id = 1");
        assert_eq!(
            event.normalized_query().as_deref(),
            Some("SELECT * FROM users WHERE id = ?")
        );
    }

    #[test]
    fn converts_duration_entries_into_duration_events() {
        let entry = entry(
            LogLevel::Duration,
            "duration: 15.234 ms",
            Some(15.234),
            None,
        );

        let event = NormalizedEvent::from_log_entry(&entry, EventSourceKind::Stderr, 1);

        assert_eq!(event.event_id, "stderr:1");
        assert!(!event.is_query());
        assert_eq!(event.duration_ms(), Some(15.234));
        assert!(matches!(
            event.kind,
            EventKind::Duration(DurationEvent {
                duration_ms: 15.234
            })
        ));
    }

    #[test]
    fn converts_error_entries_into_error_events() {
        let entry = entry(
            LogLevel::Error,
            "relation \"missing_table\" does not exist",
            None,
            None,
        );

        let event = NormalizedEvent::from_log_entry(&entry, EventSourceKind::Stderr, 2);

        assert!(event.is_error());
        assert_eq!(event.message(), "relation \"missing_table\" does not exist");
        match event.kind {
            EventKind::Error(error) => {
                assert_eq!(error.message, "relation \"missing_table\" does not exist");
                assert_eq!(error.sqlstate, None);
            }
            other => panic!("expected error event, got {other:?}"),
        }
    }

    #[test]
    fn converts_non_special_entries_into_log_events() {
        let entry = entry(
            LogLevel::Warning,
            "there is no transaction in progress",
            None,
            None,
        );

        let event = NormalizedEvent::from_log_entry(&entry, EventSourceKind::Stderr, 3);

        assert!(!event.is_query());
        assert!(!event.is_error());
        assert_eq!(event.message(), "there is no transaction in progress");
        match event.kind {
            EventKind::Log { level, message } => {
                assert_eq!(level, LogLevel::Warning);
                assert_eq!(message, "there is no transaction in progress");
            }
            other => panic!("expected log event, got {other:?}"),
        }
    }

    #[test]
    fn normalizes_log_entries_with_stable_source_references() {
        let entries = vec![
            entry(LogLevel::Log, "connection received", None, None),
            entry(
                LogLevel::Statement,
                "statement: SELECT 1",
                Some(1.5),
                crate::Query::from_sql("SELECT 1").ok(),
            ),
            entry(LogLevel::Duration, "duration: 1.5 ms", Some(1.5), None),
        ];

        let events = normalize_log_entries(&entries, EventSourceKind::Stderr);

        assert_eq!(events.len(), 3);
        assert_eq!(events[0].event_id, "stderr:0");
        assert_eq!(events[1].event_id, "stderr:1");
        assert_eq!(events[2].event_id, "stderr:2");
        assert_eq!(events[0].source.record_index, 0);
        assert_eq!(events[1].source.record_index, 1);
        assert_eq!(events[2].source.record_index, 2);
    }

    #[test]
    fn event_ids_include_source_kind_prefixes() {
        let entry = entry(LogLevel::Log, "checkpoint complete", None, None);

        let csv_event = NormalizedEvent::from_log_entry(&entry, EventSourceKind::Csvlog, 4);
        let json_event = NormalizedEvent::from_log_entry(&entry, EventSourceKind::Jsonlog, 5);

        assert_eq!(csv_event.event_id, "csvlog:4");
        assert_eq!(json_event.event_id, "jsonlog:5");
    }
}
