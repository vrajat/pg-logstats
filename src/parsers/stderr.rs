//! PostgreSQL stderr log format parser
//!
//! Handles PostgreSQL 17 stderr logs with standard log_line_prefix = '%m [%p] %q%u@%d %a: '

use crate::{timestamp_error, LogEntry, LogLevel, PgLogstatsError, Result};
use chrono::{DateTime, Utc};
use regex::Regex;
use sqlparser::ast::{Expr, Value, VisitMut, VisitorMut};
use sqlparser::dialect::PostgreSqlDialect;
use sqlparser::parser::Parser;

/// Parser for PostgreSQL stderr log format
pub struct StderrParser {
    pub log_line_regex: Regex,
    duration_regex: Regex,
    parameter_regex: Regex,
    // State for handling multi-line statements
    pending_statement: Option<PendingStatement>,
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

impl StderrParser {
    /// Create a new stderr parser
    pub fn new() -> Self {
        Self {
            log_line_regex: Regex::new(
                r"^(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}(?:\.\d+)?) (\w+) \[(\d+)\] ([^@]+)@([^ ]+) ([^:]+): (\w+):\s*(.+)$"
            ).unwrap(),
            duration_regex: Regex::new(r"duration: ([\d.]+) ms").unwrap(),
            parameter_regex: Regex::new(r"\$(\d+)").unwrap(),
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

        // Try to parse as a standard log line
        if let Some(captures) = self.log_line_regex.captures(line) {
            return self.parse_standard_format(&captures, line);
        }

        // If we can't parse it, return None (skip unparseable lines)
        Ok(None)
    }

    /// Parse multiple log lines with state management
    pub fn parse_lines(&self, lines: &[String]) -> Result<Vec<LogEntry>> {
        let mut parser = StderrParser::new();
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
                query: Some(pending.query),
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

    /// Parse standard PostgreSQL log format
    fn parse_standard_format(
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
        let message_type = LogLevel::from(log_level);

        // Determine the actual message type based on content
        let actual_message_type = if message.starts_with("statement: ") {
            LogLevel::Statement
        } else if message.starts_with("duration: ") {
            LogLevel::Duration
        } else {
            message_type
        };

        // Handle different message types
        match actual_message_type {
            LogLevel::Statement => self
                .handle_statement_message(timestamp, process_id, user, database, app_name, message),
            LogLevel::Duration => self
                .handle_duration_message(timestamp, process_id, user, database, app_name, message),
            _ => {
                // Handle other log levels (ERROR, WARNING, etc.)
                let entry = LogEntry {
                    timestamp,
                    process_id: process_id.to_string(),
                    user: Some(user.to_string()),
                    database: Some(database.to_string()),
                    client_host: None,
                    application_name: Some(app_name.to_string()),
                    message_type: actual_message_type,
                    message: message.to_string(),
                    query: None,
                    duration: None,
                };
                Ok(Some(entry))
            }
        }
    }

    /// Handle statement messages (may be multi-line)
    fn handle_statement_message(
        &mut self,
        timestamp: DateTime<Utc>,
        process_id: &str,
        user: &str,
        database: &str,
        app_name: &str,
        message: &str,
    ) -> Result<Option<LogEntry>> {
        // Extract the actual query from "statement: SELECT ..."
        let query = if message.starts_with("statement: ") {
            &message[10..]
        } else {
            message
        };

        // For now, always create a statement entry
        // Multi-line handling will be done by continuation lines
        let normalized_query = match self.normalize_query(query) {
            Ok(q) => Some(q),
            Err(e) => {
                eprintln!("Failed to normalize query: {}", e);
                None
            }
        };
        let entry = LogEntry {
            timestamp,
            process_id: process_id.to_string(),
            user: Some(user.to_string()),
            database: Some(database.to_string()),
            client_host: None,
            application_name: Some(app_name.to_string()),
            message_type: LogLevel::Statement,
            message: format!("statement: {}", query),
            query: normalized_query,
            duration: None,
        };
        Ok(Some(entry))
    }

    /// Handle duration messages
    fn handle_duration_message(
        &mut self,
        timestamp: DateTime<Utc>,
        process_id: &str,
        user: &str,
        database: &str,
        app_name: &str,
        message: &str,
    ) -> Result<Option<LogEntry>> {
        if let Some(duration) = self.extract_duration(message) {
            // For now, create a standalone duration entry
            // In a more sophisticated implementation, we would track the last statement
            // and associate the duration with it
            let entry = LogEntry {
                timestamp,
                process_id: process_id.to_string(),
                user: Some(user.to_string()),
                database: Some(database.to_string()),
                client_host: None,
                application_name: Some(app_name.to_string()),
                message_type: LogLevel::Duration,
                message: message.to_string(),
                query: None,
                duration: Some(duration),
            };
            Ok(Some(entry))
        } else {
            // Duration message without valid duration
            let entry = LogEntry {
                timestamp,
                process_id: process_id.to_string(),
                user: Some(user.to_string()),
                database: Some(database.to_string()),
                client_host: None,
                application_name: Some(app_name.to_string()),
                message_type: LogLevel::Duration,
                message: message.to_string(),
                query: None,
                duration: None,
            };
            Ok(Some(entry))
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

    /// Normalize SQL query by replacing parameters with placeholders (public for testing)
    pub fn normalize_query(&self, query: &str) -> Result<String> {
        let dialect = PostgreSqlDialect {};

        // Parse the SQL query
        let mut ast = Parser::parse_sql(&dialect, query).map_err(|e| PgLogstatsError::Parse {
            message: format!("Failed to parse SQL: {}", e),
            line_number: None,
            line_content: Some(query.to_string()),
        })?;

        if ast.is_empty() {
            return Ok(query.to_string());
        }

        // Create visitor to normalize literals
        let mut normalizer = LiteralNormalizer;

        // Apply normalization to all statements
        for stmt in &mut ast {
            let _ = stmt.visit(&mut normalizer);
        }

        // Convert back to SQL string
        let normalized_sql = ast
            .iter()
            .map(|stmt| stmt.to_string())
            .collect::<Vec<_>>()
            .join("; ");

        Ok(normalized_sql)
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

/// Visitor that replaces literal values with placeholders
struct LiteralNormalizer;

impl VisitorMut for LiteralNormalizer {
    type Break = ();

    fn pre_visit_expr(&mut self, _expr: &mut Expr) -> std::ops::ControlFlow<Self::Break> {
        // Always continue traversal to visit nested expressions
        std::ops::ControlFlow::Continue(())
    }

    fn post_visit_expr(&mut self, expr: &mut Expr) -> std::ops::ControlFlow<Self::Break> {
        match expr {
            // Replace literal constants with placeholders
            Expr::Value(Value::Number(_, _))
            | Expr::Value(Value::SingleQuotedString(_))
            | Expr::Value(Value::DoubleQuotedString(_))
            | Expr::Value(Value::Boolean(_))
            | Expr::Value(Value::Null) => {
                *expr = Expr::Value(Value::Placeholder("?".to_string()));
            }

            // Normalize existing parameters to standard format
            Expr::Value(Value::Placeholder(_)) => {
                *expr = Expr::Value(Value::Placeholder("?".to_string()));
            }

            // Continue traversing for all other expressions
            _ => {}
        }

        std::ops::ControlFlow::Continue(())
    }
}

impl Default for StderrParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_statement() {
        let mut parser = StderrParser::new();
        let line = "2024-08-14 10:30:15.123 UTC [12345] postgres@testdb psql: LOG:  statement: SELECT * FROM users WHERE active = true;";

        let result = parser.parse_line(line).unwrap();
        assert!(result.is_some());

        let entry = result.unwrap();
        assert_eq!(entry.process_id, "12345");
        assert_eq!(entry.user, Some("postgres".to_string()));
        assert_eq!(entry.database, Some("testdb".to_string()));
        assert_eq!(entry.application_name, Some("psql".to_string()));
        assert_eq!(entry.message_type, LogLevel::Statement);
        assert!(entry.query.is_some());
        assert_eq!(entry.query.unwrap(), "SELECT * FROM users WHERE active = ?");
    }

    #[test]
    fn test_parse_duration() {
        let mut parser = StderrParser::new();
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
        let mut parser = StderrParser::new();
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
        let mut parser = StderrParser::new();
        let line = "2024-08-14 10:30:17.012 UTC [12347] postgres@testdb psql: LOG:  statement: UPDATE products SET price = $1 WHERE id = $2";

        let result = parser.parse_line(line).unwrap();
        assert!(result.is_some());

        let entry = result.unwrap();
        assert_eq!(
            entry.query,
            Some("UPDATE products SET price = ? WHERE id = ?".to_string())
        );
    }

    #[test]
    fn test_parse_multi_line_statement() {
        let lines = vec![
            "2024-08-14 10:30:18.000 UTC [12348] postgres@testdb psql: LOG:  statement: SELECT u.name, p.title",
            "    FROM users u",
            "    JOIN posts p ON u.id = p.user_id",
            "    WHERE u.active = true",
            "    ORDER BY p.created_at DESC;",
            "2024-08-14 10:30:18.123 UTC [12348] postgres@testdb psql: LOG:  duration: 12.345 ms",
        ];

        let parser = StderrParser::new();
        let result = parser.parse_lines(&lines.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert!(result.is_ok());

        let entries = result.unwrap();
        assert_eq!(entries.len(), 2); // Should parse 2 entries: statement and duration
        let statement_entry = &entries[0];
        let duration_entry = &entries[1];
        assert_eq!(statement_entry.message_type, LogLevel::Statement);
        assert_eq!(duration_entry.message_type, LogLevel::Duration);
        assert_eq!(duration_entry.duration, Some(12.345));
        assert!(statement_entry
            .query
            .as_ref()
            .unwrap()
            .contains("SELECT u.name, p.title"));
    }

    #[test]
    fn test_parse_empty_line() {
        let mut parser = StderrParser::new();
        let result = parser.parse_line("").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_unparseable_line() {
        let mut parser = StderrParser::new();
        let result = parser
            .parse_line("This is not a PostgreSQL log line")
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_lines_with_errors() {
        let lines = vec![
            "2024-08-14 10:30:15.123 UTC [12345] postgres@testdb psql: LOG:  statement: SELECT * FROM users;",
            "This is not a valid log line",
            "2024-08-14 10:30:15.456 UTC [12345] postgres@testdb psql: LOG:  duration: 45.123 ms",
        ];

        let parser = StderrParser::new();
        let result = parser.parse_lines(&lines.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert!(result.is_ok());

        let entries = result.unwrap();
        assert_eq!(entries.len(), 2); // Should parse 2 valid lines, skip 1 invalid
    }

    #[test]
    fn test_normalize_query() {
        let parser = StderrParser::new();

        // Test parameter replacement
        let query = "UPDATE users SET name = $1, email = $2 WHERE id = $3";
        let normalized = parser.normalize_query(query).unwrap();
        assert_eq!(
            normalized,
            "UPDATE users SET name = ?, email = ? WHERE id = ?"
        );

        // Test whitespace normalization
        let query = "SELECT   *   FROM    users   WHERE   id=1";
        let normalized = parser.normalize_query(query).unwrap();
        assert_eq!(normalized, "SELECT * FROM users WHERE id = ?");
    }

    #[test]
    fn test_normalize_simple_query() {
        let parser = StderrParser::new();

        let input = "SELECT * FROM users WHERE name = 'John' AND city = 'New York'";
        let result = parser.normalize_query(input).unwrap();

        // Should be: "SELECT * FROM users WHERE name = ? AND city = ?"
        assert!(result.contains("name = ?"));
        assert!(result.contains("city = ?"));
        assert!(!result.contains("'John'"));
        assert!(!result.contains("'New York'"));
    }

    #[test]
    fn test_normalize_complex_query() {
        let parser = StderrParser::new();

        let input = "SELECT * FROM users WHERE (age > 25 AND name = 'John') OR id IN (1, 2, 3)";
        let result = parser.normalize_query(input).unwrap();

        assert!(result.contains("age > ?"));
        assert!(result.contains("name = ?"));
        assert!(result.contains("IN (?, ?, ?)"));
    }

    #[test]
    fn test_timestamp_parsing() {
        let parser = StderrParser::new();

        // Test timestamp parsing
        let timestamp_str = "2024-08-14 10:30:15.123";
        let timezone = "UTC";

        let result = parser.parse_timestamp(timestamp_str, timezone);
        println!("Timestamp parsing result: {:?}", result);
        assert!(result.is_ok());
    }

    #[test]
    fn test_regex_matching() {
        let parser = StderrParser::new();

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
