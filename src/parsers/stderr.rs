//! PostgreSQL stderr log format parser

use crate::{LogEntry, LogLevel, PgLoggrepError, Result, parse_error, timestamp_error};
use chrono::{DateTime, Utc};
use regex::Regex;

/// Parser for PostgreSQL stderr log format
pub struct StderrParser {
    // Regex patterns for parsing different log formats
    timestamp_regex: Regex,
    log_line_regex: Regex,
}

impl StderrParser {
    /// Create a new stderr parser
    pub fn new() -> Self {
        Self {
            timestamp_regex: Regex::new(r"(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}(?:\.\d+)?)").unwrap(),
            log_line_regex: Regex::new(r"^(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}(?:\.\d+)?) (\w+) \[(\d+)\]: \[(\d+)-(\d+)\] user=([^,]+),db=([^,]+),app=([^,]+),client=([^:]+):\d+ (\w+): (.+)$").unwrap(),
        }
    }

    /// Parse a single log line
    pub fn parse_line(&self, line: &str) -> Result<LogEntry> {
        if line.trim().is_empty() {
            return Err(parse_error("Empty log line", None, Some(line)));
        }

        // Try to match the standard PostgreSQL log format
        if let Some(captures) = self.log_line_regex.captures(line) {
            return self.parse_standard_format(&captures, line);
        }

        // Try to parse as a simple timestamp + message format
        if let Some(captures) = self.timestamp_regex.captures(line) {
            return self.parse_simple_format(&captures, line);
        }

        Err(parse_error("Unrecognized log format", None, Some(line)))
    }

    /// Parse multiple log lines
    pub fn parse_lines(&self, lines: &[String]) -> Result<Vec<LogEntry>> {
        let mut entries = Vec::new();
        let mut errors = Vec::new();

        for (line_number, line) in lines.iter().enumerate() {
            match self.parse_line(line) {
                Ok(entry) => entries.push(entry),
                Err(e) => {
                    errors.push(format!("Line {}: {}", line_number + 1, e));
                }
            }
        }

        if !errors.is_empty() {
            return Err(PgLoggrepError::Parse {
                message: format!("Failed to parse {} lines: {}", errors.len(), errors.join("; ")),
                line_number: None,
                line_content: None,
            });
        }

        Ok(entries)
    }

    /// Parse standard PostgreSQL log format
    fn parse_standard_format(&self, captures: &regex::Captures, _original_line: &str) -> Result<LogEntry> {
        let timestamp_str = captures.get(1).unwrap().as_str();
        let timezone = captures.get(2).unwrap().as_str();
        let process_id = captures.get(3).unwrap().as_str();
        let user = captures.get(6).unwrap().as_str();
        let database = captures.get(7).unwrap().as_str();
        let app_name = captures.get(8).unwrap().as_str();
        let client_host = captures.get(9).unwrap().as_str();
        let log_level = captures.get(10).unwrap().as_str();
        let message = captures.get(11).unwrap().as_str();

        let timestamp = self.parse_timestamp(timestamp_str, timezone)?;
        let message_type = LogLevel::from(log_level);

        let mut entry = LogEntry::new(
            timestamp,
            process_id.to_string(),
            message_type.clone(),
            message.to_string(),
        );

        entry.user = Some(user.to_string());
        entry.database = Some(database.to_string());
        entry.application_name = Some(app_name.to_string());
        entry.client_host = Some(client_host.to_string());

        // Extract query if this is a statement
        if message_type == LogLevel::Statement {
            entry.query = Some(message.to_string());
        }

        // Extract duration if this is a duration message
        if message_type == LogLevel::Duration {
            if let Some(duration) = self.extract_duration(message) {
                entry.duration = Some(duration);
            }
        }

        Ok(entry)
    }

    /// Parse simple timestamp + message format
    fn parse_simple_format(&self, captures: &regex::Captures, original_line: &str) -> Result<LogEntry> {
        let timestamp_str = captures.get(1).unwrap().as_str();
        let timestamp = self.parse_timestamp(timestamp_str, "UTC")?;

        // Extract the rest of the line as the message
        let message_start = captures.get(0).unwrap().end();
        let message = original_line[message_start..].trim();

        Ok(LogEntry::new(
            timestamp,
            "unknown".to_string(),
            LogLevel::Log,
            message.to_string(),
        ))
    }

    /// Parse timestamp string into DateTime<Utc>
    fn parse_timestamp(&self, timestamp_str: &str, timezone: &str) -> Result<DateTime<Utc>> {
        // Try parsing with milliseconds
        if let Ok(dt) = DateTime::parse_from_str(&format!("{} {}", timestamp_str, timezone), "%Y-%m-%d %H:%M:%S%.f %Z") {
            return Ok(dt.with_timezone(&Utc));
        }

        // Try parsing without milliseconds
        if let Ok(dt) = DateTime::parse_from_str(&format!("{} {}", timestamp_str, timezone), "%Y-%m-%d %H:%M:%S %Z") {
            return Ok(dt.with_timezone(&Utc));
        }

        Err(timestamp_error("Failed to parse timestamp", timestamp_str))
    }

    /// Extract duration from duration message
    fn extract_duration(&self, message: &str) -> Option<f64> {
        // Look for patterns like "duration: 123.456 ms"
        let duration_regex = Regex::new(r"duration: ([\d.]+) ms").ok()?;
        duration_regex.captures(message)
            .and_then(|captures| captures.get(1))
            .and_then(|m| m.as_str().parse::<f64>().ok())
    }
}

impl Default for StderrParser {
    fn default() -> Self {
        Self::new()
    }
}
