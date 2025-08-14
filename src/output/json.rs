//! JSON output formatter for pg-loggrep results

use crate::analytics::{QueryAnalysis, TimingAnalysis};
use crate::parsers::LogEntry;
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
    pub fn format_query_analysis(&self, analysis: &QueryAnalysis) -> Result<String, String> {
        let json_value = json!({
            "total_queries": analysis.total_queries,
            "slow_queries_count": analysis.slow_queries.len(),
            "frequent_queries": analysis.frequent_queries,
            "query_types": analysis.query_types,
        });

        serde_json::to_string_pretty(&json_value)
            .map_err(|e| format!("Failed to serialize JSON: {}", e))
    }

    /// Format timing analysis results as JSON
    pub fn format_timing_analysis(&self, analysis: &TimingAnalysis) -> Result<String, String> {
        let json_value = json!({
            "average_response_time_ms": analysis.average_response_time.num_milliseconds(),
            "p95_response_time_ms": analysis.p95_response_time.num_milliseconds(),
            "p99_response_time_ms": analysis.p99_response_time.num_milliseconds(),
            "hourly_patterns": analysis.hourly_patterns,
            "daily_patterns": analysis.daily_patterns,
        });

        serde_json::to_string_pretty(&json_value)
            .map_err(|e| format!("Failed to serialize JSON: {}", e))
    }

    /// Format log entries as JSON
    pub fn format_log_entries(&self, entries: &[LogEntry]) -> Result<String, String> {
        let json_value = json!(entries);

        serde_json::to_string_pretty(&json_value)
            .map_err(|e| format!("Failed to serialize JSON: {}", e))
    }
}

impl Default for JsonFormatter {
    fn default() -> Self {
        Self::new()
    }
}
