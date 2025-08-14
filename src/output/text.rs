//! Human-readable text output formatter for pg-loggrep results

use crate::{AnalysisResult, TimingAnalysis, LogEntry, PgLoggrepError, Result};
use std::fmt::Write;

/// Text formatter for analysis results
pub struct TextFormatter {
    // Configuration for text formatting
}

impl TextFormatter {
    /// Create a new text formatter
    pub fn new() -> Self {
        Self {}
    }

    /// Format query analysis results as text
    pub fn format_query_analysis(&self, analysis: &AnalysisResult) -> Result<String> {
        let mut output = String::new();

        writeln!(output, "Query Analysis Report").map_err(|e| PgLoggrepError::Unexpected { message: e.to_string(), context: Some("text formatting".to_string()) })?;
        writeln!(output, "===================").map_err(|e| PgLoggrepError::Unexpected { message: e.to_string(), context: Some("text formatting".to_string()) })?;
        writeln!(output, "Total Queries: {}", analysis.total_queries).map_err(|e| PgLoggrepError::Unexpected { message: e.to_string(), context: Some("text formatting".to_string()) })?;
        writeln!(output, "Total Duration: {:.2} ms", analysis.total_duration).map_err(|e| PgLoggrepError::Unexpected { message: e.to_string(), context: Some("text formatting".to_string()) })?;
        writeln!(output, "Average Duration: {:.2} ms", analysis.average_duration).map_err(|e| PgLoggrepError::Unexpected { message: e.to_string(), context: Some("text formatting".to_string()) })?;
        writeln!(output, "P95 Duration: {:.2} ms", analysis.p95_duration).map_err(|e| PgLoggrepError::Unexpected { message: e.to_string(), context: Some("text formatting".to_string()) })?;
        writeln!(output, "P99 Duration: {:.2} ms", analysis.p99_duration).map_err(|e| PgLoggrepError::Unexpected { message: e.to_string(), context: Some("text formatting".to_string()) })?;
        writeln!(output, "Error Count: {}", analysis.error_count).map_err(|e| PgLoggrepError::Unexpected { message: e.to_string(), context: Some("text formatting".to_string()) })?;
        writeln!(output, "Connection Count: {}", analysis.connection_count).map_err(|e| PgLoggrepError::Unexpected { message: e.to_string(), context: Some("text formatting".to_string()) })?;

        if !analysis.query_types.is_empty() {
            writeln!(output, "\nQuery Types:").map_err(|e| PgLoggrepError::Unexpected { message: e.to_string(), context: Some("text formatting".to_string()) })?;
            for (query_type, count) in &analysis.query_types {
                writeln!(output, "  {}: {}", query_type, count).map_err(|e| PgLoggrepError::Unexpected { message: e.to_string(), context: Some("text formatting".to_string()) })?;
            }
        }

        if !analysis.slowest_queries.is_empty() {
            writeln!(output, "\nSlowest Queries:").map_err(|e| PgLoggrepError::Unexpected { message: e.to_string(), context: Some("text formatting".to_string()) })?;
            for (i, (query, duration)) in analysis.slowest_queries.iter().enumerate() {
                writeln!(output, "  {}. {:.2} ms: {}", i + 1, duration, query).map_err(|e| PgLoggrepError::Unexpected { message: e.to_string(), context: Some("text formatting".to_string()) })?;
            }
        }

        Ok(output)
    }

    /// Format timing analysis results as text
    pub fn format_timing_analysis(&self, analysis: &TimingAnalysis) -> Result<String> {
        let mut output = String::new();

        writeln!(output, "Timing Analysis Report").map_err(|e| PgLoggrepError::Unexpected { message: e.to_string(), context: Some("text formatting".to_string()) })?;
        writeln!(output, "====================").map_err(|e| PgLoggrepError::Unexpected { message: e.to_string(), context: Some("text formatting".to_string()) })?;
        writeln!(output, "Average Response Time: {}ms",
            analysis.average_response_time.num_milliseconds()).map_err(|e| PgLoggrepError::Unexpected { message: e.to_string(), context: Some("text formatting".to_string()) })?;
        writeln!(output, "95th Percentile: {}ms",
            analysis.p95_response_time.num_milliseconds()).map_err(|e| PgLoggrepError::Unexpected { message: e.to_string(), context: Some("text formatting".to_string()) })?;
        writeln!(output, "99th Percentile: {}ms",
            analysis.p99_response_time.num_milliseconds()).map_err(|e| PgLoggrepError::Unexpected { message: e.to_string(), context: Some("text formatting".to_string()) })?;

        Ok(output)
    }

    /// Format log entries as text
    pub fn format_log_entries(&self, entries: &[LogEntry]) -> Result<String> {
        let mut output = String::new();

        writeln!(output, "Log Entries ({} total)", entries.len()).map_err(|e| PgLoggrepError::Unexpected { message: e.to_string(), context: Some("text formatting".to_string()) })?;
        writeln!(output, "================").map_err(|e| PgLoggrepError::Unexpected { message: e.to_string(), context: Some("text formatting".to_string()) })?;

        for (i, entry) in entries.iter().enumerate() {
            writeln!(output, "[{}] {} {}: {}",
                i + 1,
                entry.timestamp.format("%Y-%m-%d %H:%M:%S"),
                entry.message_type,
                entry.message
            ).map_err(|e| PgLoggrepError::Unexpected { message: e.to_string(), context: Some("text formatting".to_string()) })?;
        }

        Ok(output)
    }
}

impl Default for TextFormatter {
    fn default() -> Self {
        Self::new()
    }
}
