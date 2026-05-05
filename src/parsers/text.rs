//! Text log format parser
//!
//! Handles the default text log prefix `log_line_prefix =
//! '%m [%p] %q%u@%d %a: '` and Amazon RDS logs with the documented RDS prefix
//! shape `%t:%r:%u@%d:[%p]:`.

use crate::{timestamp_error, LogEntry, LogLevel, PgLogstatsError, Result};
use chrono::{DateTime, Utc};
use regex::Regex;

/// Text log prefix variants supported by the parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextLogFormat {
    /// Accept any supported text log prefix.
    Auto,
    /// Default local text prefix used by pg-logstats fixtures.
    Default,
    /// Amazon RDS prefix `%t:%r:%u@%d:[%p]:`.
    AwsRds,
}

impl TextLogFormat {
    fn accepts_default(self) -> bool {
        matches!(self, Self::Auto | Self::Default)
    }

    fn accepts_rds(self) -> bool {
        matches!(self, Self::Auto | Self::AwsRds)
    }
}

/// Parser for supported text log formats.
pub struct TextLogParser {
    pub log_line_regex: Regex,
    pub rds_log_line_regex: Regex,
    duration_regex: Regex,
    duration_statement_regex: Regex,
    execute_statement_regex: Regex,
    parameter_regex: Regex,
    format: TextLogFormat,
    // State for handling multi-line statements
    pending_statement: Option<PendingStatement>,
}

#[derive(Debug, Clone)]
struct LogMetadata {
    process_id: String,
    user: Option<String>,
    database: Option<String>,
    client_host: Option<String>,
    application_name: Option<String>,
}

/// Represents a statement that spans multiple lines
#[derive(Debug)]
struct PendingStatement {
    timestamp: DateTime<Utc>,
    process_id: String,
    user: String,
    database: String,
    application_name: String,
    query: String,
    line_count: usize,
}

impl TextLogParser {
    /// Create a new text log parser.
    pub fn new() -> Self {
        Self::with_format(TextLogFormat::Auto)
    }

    /// Create a parser restricted to one supported text log prefix.
    pub fn with_format(format: TextLogFormat) -> Self {
        Self {
            log_line_regex: Regex::new(
                r"^(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}(?:\.\d+)?) ([A-Za-z0-9_+\-:/]+) \[(\d+)\] ([^@]+)@([^ ]+) ([^:]+): (\w+):\s*(.+)$"
            ).unwrap(),
            rds_log_line_regex: Regex::new(
                r"^(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}(?:\.\d+)?)(?: ([^:]+))?:(.*):([^@]+)@([^:]+):\[(\d+)\]:(\w+):\s*(.+)$"
            ).unwrap(),
            duration_regex: Regex::new(r"duration:\s*([\d.]+)\s*ms").unwrap(),
            duration_statement_regex: Regex::new(
                r"^duration:\s*([\d.]+)\s*ms\s+(?:statement|execute\s+[^:]+):\s*(.+)$"
            )
            .unwrap(),
            execute_statement_regex: Regex::new(r"^execute\s+[^:]+:\s*(.+)$").unwrap(),
            parameter_regex: Regex::new(r"\$(\d+)").unwrap(),
            format,
            pending_statement: None,
        }
    }

    /// Parse a single log line
    /// Returns Ok(Some(LogEntry)) for valid log entries
    /// Returns Ok(None) for unparseable lines (continuation lines, empty lines, etc.)
    /// Returns Err for critical parsing errors
    pub fn parse_line(&mut self, line: &str) -> Result<Option<LogEntry>> {
        let line = line.trim();

        // Skip empty lines
        if line.is_empty() {
            return Ok(None);
        }

        // Check if this is a continuation line (no timestamp)
        if !line.chars().next().unwrap_or(' ').is_ascii_digit() {
            return self.handle_continuation_line(line);
        }

        // Try to parse as the default local text log line.
        if self.format.accepts_default() {
            if let Some(captures) = self.log_line_regex.captures(line) {
                return self.parse_default_format(&captures, line);
            }
        }

        // Try to parse as an Amazon RDS PostgreSQL stderr log line.
        if self.format.accepts_rds() {
            if let Some(captures) = self.rds_log_line_regex.captures(line) {
                return self.parse_rds_format(&captures, line);
            }
        }

        // If we can't parse it, return None (skip unparseable lines)
        Ok(None)
    }

    /// Parse multiple log lines with state management
    pub fn parse_lines(&self, lines: &[String]) -> Result<Vec<LogEntry>> {
        let mut parser = TextLogParser::with_format(self.format);
        let mut entries = Vec::new();
        let mut errors = Vec::new();

        for (line_number, line) in lines.iter().enumerate() {
            match parser.parse_line(line) {
                Ok(Some(entry)) => entries.push(entry),
                Ok(None) => {
                    // Skip unparseable lines silently
                }
                Err(e) => {
                    errors.push(format!("Line {}: {}", line_number + 1, e));
                }
            }
        }

        // If we have a pending statement, finalize it
        if let Some(pending) = parser.pending_statement.take() {
            entries.push(LogEntry {
                timestamp: pending.timestamp,
                process_id: pending.process_id,
                user: Some(pending.user),
                database: Some(pending.database),
                client_host: None,
                application_name: Some(pending.application_name),
                message_type: LogLevel::Statement,
                message: format!("statement: {}", pending.query),
                queries: crate::Query::from_sql(&pending.query).ok(),
                duration: None,
            });
        }

        if !errors.is_empty() {
            return Err(PgLogstatsError::Parse {
                message: format!(
                    "Failed to parse {} lines: {}",
                    errors.len(),
                    errors.join("; ")
                ),
                line_number: None,
                line_content: None,
            });
        }

        Ok(entries)
    }

    /// Parse the default text log format.
    fn parse_default_format(
        &mut self,
        captures: &regex::Captures,
        _original_line: &str,
    ) -> Result<Option<LogEntry>> {
        let timestamp_str = captures.get(1).unwrap().as_str();
        let timezone = captures.get(2).unwrap().as_str();
        let process_id = captures.get(3).unwrap().as_str();
        let user = captures.get(4).unwrap().as_str();
        let database = captures.get(5).unwrap().as_str();
        let app_name = captures.get(6).unwrap().as_str();
        let log_level = captures.get(7).unwrap().as_str();
        let message = captures.get(8).unwrap().as_str();

        let timestamp = self.parse_timestamp(timestamp_str, timezone)?;
        let metadata =
            LogMetadata::new(process_id, Some(user), Some(database), None, Some(app_name));

        self.parse_message(timestamp, metadata, log_level, message)
    }

    /// Parse Amazon RDS PostgreSQL log format.
    fn parse_rds_format(
        &mut self,
        captures: &regex::Captures,
        _original_line: &str,
    ) -> Result<Option<LogEntry>> {
        let timestamp_str = captures.get(1).unwrap().as_str();
        let timezone = captures.get(2).map(|m| m.as_str()).unwrap_or("UTC");
        let remote_host = captures.get(3).unwrap().as_str();
        let user = captures.get(4).unwrap().as_str();
        let database = captures.get(5).unwrap().as_str();
        let process_id = captures.get(6).unwrap().as_str();
        let log_level = captures.get(7).unwrap().as_str();
        let message = captures.get(8).unwrap().as_str();

        let timestamp = self.parse_timestamp(timestamp_str, timezone)?;
        let metadata = LogMetadata::new(
            process_id,
            Some(user),
            Some(database),
            normalize_rds_client_host(remote_host),
            None,
        );

        self.parse_message(timestamp, metadata, log_level, message)
    }

    fn parse_message(
        &mut self,
        timestamp: DateTime<Utc>,
        metadata: LogMetadata,
        log_level: &str,
        message: &str,
    ) -> Result<Option<LogEntry>> {
        if let Some((duration_ms, statement)) = self.extract_duration_statement(message) {
            return self.handle_statement_message(
                timestamp,
                metadata,
                statement,
                Some(duration_ms),
            );
        }

        if let Some(statement) = self.extract_statement(message) {
            return self.handle_statement_message(timestamp, metadata, statement, None);
        }

        if message.starts_with("duration: ") {
            return self.handle_duration_message(timestamp, metadata, message);
        }

        Ok(Some(metadata.into_entry(
            timestamp,
            LogLevel::from(log_level),
            message.to_string(),
            None,
            None,
        )))
    }

    /// Handle statement messages (may be multi-line)
    fn handle_statement_message(
        &mut self,
        timestamp: DateTime<Utc>,
        metadata: LogMetadata,
        query: &str,
        duration_ms: Option<f64>,
    ) -> Result<Option<LogEntry>> {
        // For now, always create a statement entry
        // Multi-line handling will be done by continuation lines
        let queries = crate::Query::from_sql(query);
        let normalized_queries = queries.ok();

        Ok(Some(metadata.into_entry(
            timestamp,
            LogLevel::Statement,
            format!("statement: {}", query),
            normalized_queries,
            duration_ms,
        )))
    }

    /// Handle duration messages
    fn handle_duration_message(
        &mut self,
        timestamp: DateTime<Utc>,
        metadata: LogMetadata,
        message: &str,
    ) -> Result<Option<LogEntry>> {
        if let Some(duration) = self.extract_duration(message) {
            // For now, create a standalone duration entry
            // In a more sophisticated implementation, we would track the last statement
            // and associate the duration with it
            Ok(Some(metadata.into_entry(
                timestamp,
                LogLevel::Duration,
                message.to_string(),
                None,
                Some(duration),
            )))
        } else {
            // Duration message without valid duration
            Ok(Some(metadata.into_entry(
                timestamp,
                LogLevel::Duration,
                message.to_string(),
                None,
                None,
            )))
        }
    }

    /// Handle continuation lines (lines without timestamps)
    fn handle_continuation_line(&mut self, line: &str) -> Result<Option<LogEntry>> {
        if let Some(pending) = &mut self.pending_statement {
            // Append to the pending statement
            pending.query.push(' ');
            pending.query.push_str(line);
            pending.line_count += 1;
            Ok(None)
        } else {
            // No pending statement, skip this line
            Ok(None)
        }
    }

    /// Parse timestamp string into DateTime<Utc> (public for testing)
    pub fn parse_timestamp(&self, timestamp_str: &str, _timezone: &str) -> Result<DateTime<Utc>> {
        // Try parsing with milliseconds
        if let Ok(dt) =
            DateTime::parse_from_str(&format!("{} UTC", timestamp_str), "%Y-%m-%d %H:%M:%S%.f %Z")
        {
            return Ok(dt.with_timezone(&Utc));
        }

        // Try parsing without milliseconds
        if let Ok(dt) =
            DateTime::parse_from_str(&format!("{} UTC", timestamp_str), "%Y-%m-%d %H:%M:%S %Z")
        {
            return Ok(dt.with_timezone(&Utc));
        }

        // Try parsing with NaiveDateTime and converting
        if let Ok(naive_dt) =
            chrono::NaiveDateTime::parse_from_str(timestamp_str, "%Y-%m-%d %H:%M:%S%.f")
        {
            return Ok(DateTime::from_naive_utc_and_offset(naive_dt, Utc));
        }

        if let Ok(naive_dt) =
            chrono::NaiveDateTime::parse_from_str(timestamp_str, "%Y-%m-%d %H:%M:%S")
        {
            return Ok(DateTime::from_naive_utc_and_offset(naive_dt, Utc));
        }

        Err(timestamp_error("Failed to parse timestamp", timestamp_str))
    }

    /// Extract duration from duration message (public for testing)
    pub fn extract_duration(&self, message: &str) -> Option<f64> {
        self.duration_regex
            .captures(message)
            .and_then(|captures| captures.get(1))
            .and_then(|m| m.as_str().parse::<f64>().ok())
    }

    fn extract_duration_statement<'a>(&self, message: &'a str) -> Option<(f64, &'a str)> {
        let captures = self.duration_statement_regex.captures(message)?;
        let duration = captures.get(1)?.as_str().parse::<f64>().ok()?;
        let statement = captures.get(2)?.as_str();
        Some((duration, statement))
    }

    fn extract_statement<'a>(&self, message: &'a str) -> Option<&'a str> {
        if let Some(statement) = message.strip_prefix("statement: ") {
            return Some(statement);
        }

        self.execute_statement_regex
            .captures(message)
            .and_then(|captures| captures.get(1))
            .map(|statement| statement.as_str())
    }

    /// Get the duration regex for testing
    pub fn duration_regex(&self) -> &Regex {
        &self.duration_regex
    }

    /// Get the parameter regex for testing
    pub fn parameter_regex(&self) -> &Regex {
        &self.parameter_regex
    }
}

impl LogMetadata {
    fn new(
        process_id: &str,
        user: Option<&str>,
        database: Option<&str>,
        client_host: Option<String>,
        application_name: Option<&str>,
    ) -> Self {
        Self {
            process_id: process_id.to_string(),
            user: user.and_then(optional_metadata_value),
            database: database.and_then(optional_metadata_value),
            client_host,
            application_name: application_name.and_then(optional_metadata_value),
        }
    }

    fn into_entry(
        self,
        timestamp: DateTime<Utc>,
        message_type: LogLevel,
        message: String,
        queries: Option<Vec<crate::Query>>,
        duration: Option<f64>,
    ) -> LogEntry {
        LogEntry {
            timestamp,
            process_id: self.process_id,
            user: self.user,
            database: self.database,
            client_host: self.client_host,
            application_name: self.application_name,
            message_type,
            message,
            queries,
            duration,
        }
    }
}

fn optional_metadata_value(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() || value == "[unknown]" || value == "-" {
        None
    } else {
        Some(value.to_string())
    }
}

fn normalize_rds_client_host(remote_host: &str) -> Option<String> {
    let remote_host = remote_host.trim();
    if remote_host.is_empty() || remote_host == "[unknown]" || remote_host == "-" {
        return None;
    }

    if let Some(open_paren) = remote_host.rfind('(') {
        if remote_host.ends_with(')') && open_paren > 0 {
            return optional_metadata_value(&remote_host[..open_paren]);
        }
    }

    optional_metadata_value(remote_host)
}

impl Default for TextLogParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_statement() {
        let mut parser = TextLogParser::new();
        let line = "2024-08-14 10:30:15.123 UTC [12345] postgres@testdb psql: LOG:  statement: SELECT * FROM users WHERE active = true;";

        let result = parser.parse_line(line).unwrap();
        assert!(result.is_some());

        let entry = result.unwrap();
        assert_eq!(entry.process_id, "12345");
        assert_eq!(entry.user, Some("postgres".to_string()));
        assert_eq!(entry.database, Some("testdb".to_string()));
        assert_eq!(entry.application_name, Some("psql".to_string()));
        assert_eq!(entry.message_type, LogLevel::Statement);
        assert!(entry.queries.is_some());
        assert_eq!(entry.queries.as_ref().unwrap().len(), 1);
        assert_eq!(
            entry.queries.as_ref().unwrap()[0].normalized_query,
            "SELECT * FROM users WHERE active = ?"
        );
    }

    #[test]
    fn test_parse_duration() {
        let mut parser = TextLogParser::new();
        let line =
            "2024-08-14 10:30:15.456 UTC [12345] postgres@testdb psql: LOG:  duration: 45.123 ms";

        let result = parser.parse_line(line).unwrap();
        assert!(result.is_some());

        let entry = result.unwrap();
        assert_eq!(entry.message_type, LogLevel::Duration);
        assert_eq!(entry.duration, Some(45.123));
    }

    #[test]
    fn test_parse_error() {
        let mut parser = TextLogParser::new();
        let line = "2024-08-14 10:30:16.789 UTC [12346] admin@analytics pgbench: ERROR:  relation \"missing_table\" does not exist";

        let result = parser.parse_line(line).unwrap();
        assert!(result.is_some());

        let entry = result.unwrap();
        assert_eq!(entry.message_type, LogLevel::Error);
        assert_eq!(entry.user, Some("admin".to_string()));
        assert_eq!(entry.database, Some("analytics".to_string()));
        assert_eq!(entry.application_name, Some("pgbench".to_string()));
        assert!(entry
            .message
            .contains("relation \"missing_table\" does not exist"));
    }

    #[test]
    fn test_parse_parameterized_query() {
        let mut parser = TextLogParser::new();
        let line = "2024-08-14 10:30:17.012 UTC [12347] postgres@testdb psql: LOG:  statement: UPDATE products SET price = $1 WHERE id = $2";

        let result = parser.parse_line(line).unwrap();
        assert!(result.is_some());

        let entry = result.unwrap();
        assert_eq!(entry.message_type, LogLevel::Statement);
        assert!(entry.queries.is_some());
        assert_eq!(entry.queries.as_ref().unwrap().len(), 1);
        assert_eq!(
            entry.queries.as_ref().unwrap()[0].sql,
            "UPDATE products SET price = $1 WHERE id = $2"
        );
    }

    #[test]
    fn test_parse_multi_line_statement() {
        let lines = [
            "2024-08-14 10:30:18.000 UTC [12348] postgres@testdb psql: LOG:  statement: SELECT u.name, p.title",
            "    FROM users u",
            "    JOIN posts p ON u.id = p.user_id",
            "    WHERE u.active = true",
            "    ORDER BY p.created_at DESC;",
            "2024-08-14 10:30:18.123 UTC [12348] postgres@testdb psql: LOG:  duration: 12.345 ms",
        ];

        let parser = TextLogParser::new();
        let result = parser.parse_lines(&lines.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert!(result.is_ok());

        let entries = result.unwrap();
        assert_eq!(entries.len(), 2); // Should parse 2 entries: statement and duration
        let statement_entry = &entries[0];
        let duration_entry = &entries[1];
        assert_eq!(statement_entry.message_type, LogLevel::Statement);
        assert_eq!(duration_entry.message_type, LogLevel::Duration);
        assert_eq!(duration_entry.duration, Some(12.345));
        assert!(statement_entry.queries.is_some());
        assert_eq!(statement_entry.queries.as_ref().unwrap().len(), 1);
        assert!(statement_entry.queries.as_ref().unwrap()[0]
            .normalized_query
            .contains("SELECT u.name, p.title"));
    }

    #[test]
    fn test_parse_empty_line() {
        let mut parser = TextLogParser::new();
        let result = parser.parse_line("").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_unparseable_line() {
        let mut parser = TextLogParser::new();
        let result = parser
            .parse_line("This is not a PostgreSQL log line")
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_lines_with_errors() {
        let lines = [
            "2024-08-14 10:30:15.123 UTC [12345] postgres@testdb psql: LOG:  statement: SELECT * FROM users;",
            "This is not a valid log line",
            "2024-08-14 10:30:15.456 UTC [12345] postgres@testdb psql: LOG:  duration: 45.123 ms",
        ];

        let parser = TextLogParser::new();
        let result = parser.parse_lines(&lines.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert!(result.is_ok());

        let entries = result.unwrap();
        assert_eq!(entries.len(), 2); // Should parse 2 valid lines, skip 1 invalid
    }

    #[test]
    fn test_timestamp_parsing() {
        let parser = TextLogParser::new();

        // Test timestamp parsing
        let timestamp_str = "2024-08-14 10:30:15.123";
        let timezone = "UTC";

        let result = parser.parse_timestamp(timestamp_str, timezone);
        println!("Timestamp parsing result: {:?}", result);
        assert!(result.is_ok());
    }

    #[test]
    fn test_regex_matching() {
        let parser = TextLogParser::new();

        let line = "2024-08-14 10:30:15.123 UTC [12345] postgres@testdb psql: LOG:  statement: SELECT * FROM users WHERE active = true;";

        if let Some(captures) = parser.log_line_regex.captures(line) {
            println!("Regex matched!");
            println!("Timestamp: {}", captures.get(1).unwrap().as_str());
            println!("Timezone: {}", captures.get(2).unwrap().as_str());
            println!("Process ID: {}", captures.get(3).unwrap().as_str());
            println!("User: {}", captures.get(4).unwrap().as_str());
            println!("Database: {}", captures.get(5).unwrap().as_str());
            println!("App: {}", captures.get(6).unwrap().as_str());
            println!("Level: {}", captures.get(7).unwrap().as_str());
            println!("Message: {}", captures.get(8).unwrap().as_str());
        } else {
            println!("Regex did not match!");
            println!("Line: {}", line);
        }
    }
}
