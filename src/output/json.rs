//! JSON output formatter for pg-loggrep results

use crate::{AnalysisResult, TimingAnalysis, LogEntry, PgLoggrepError, Result};
use serde_json::json;

/// JSON formatter for analysis results
pub struct JsonFormatter {
    // Configuration for JSON formatting
}

impl JsonFormatter {
    /// Create a new JSON formatter
    pub fn new() -> Self {
        Self {}
    }

    /// Format query analysis results as JSON
    pub fn format_query_analysis(&self, analysis: &AnalysisResult) -> Result<String> {
        let json_value = json!({
            "total_queries": analysis.total_queries,
            "total_duration": analysis.total_duration,
            "average_duration": analysis.average_duration,
            "p95_duration": analysis.p95_duration,
            "p99_duration": analysis.p99_duration,
            "slow_queries_count": analysis.slowest_queries.len(),
            "slowest_queries": analysis.slowest_queries,
            "most_frequent_queries": analysis.most_frequent_queries,
            "query_types": analysis.query_types,
            "error_count": analysis.error_count,
            "connection_count": analysis.connection_count,
        });

        serde_json::to_string_pretty(&json_value)
            .map_err(|e| PgLoggrepError::Serialization(e))
    }

    /// Format timing analysis results as JSON
    pub fn format_timing_analysis(&self, analysis: &TimingAnalysis) -> Result<String> {
        let json_value = json!({
            "average_response_time_ms": analysis.average_response_time.num_milliseconds(),
            "p95_response_time_ms": analysis.p95_response_time.num_milliseconds(),
            "p99_response_time_ms": analysis.p99_response_time.num_milliseconds(),
            "hourly_patterns": analysis.hourly_patterns,
            "daily_patterns": analysis.daily_patterns,
        });

        serde_json::to_string_pretty(&json_value)
            .map_err(|e| PgLoggrepError::Serialization(e))
    }

    /// Format log entries as JSON
    pub fn format_log_entries(&self, entries: &[LogEntry]) -> Result<String> {
        let json_value = json!(entries);

        serde_json::to_string_pretty(&json_value)
            .map_err(|e| PgLoggrepError::Serialization(e))
    }
}

impl Default for JsonFormatter {
    fn default() -> Self {
        Self::new()
    }
}
