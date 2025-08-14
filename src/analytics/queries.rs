//! Query analysis functionality for PostgreSQL logs

use crate::{LogEntry, AnalysisResult, Result};
use std::collections::HashMap;

/// Analyzer for SQL queries found in PostgreSQL logs
pub struct QueryAnalyzer {
    // Configuration and state for query analysis
    slow_query_threshold: f64,
    max_slow_queries: usize,
    max_frequent_queries: usize,
}

impl QueryAnalyzer {
    /// Create a new query analyzer
    pub fn new() -> Self {
        Self {
            slow_query_threshold: 1000.0, // 1 second default
            max_slow_queries: 10,
            max_frequent_queries: 20,
        }
    }

    /// Create a new query analyzer with custom settings
    pub fn with_settings(slow_query_threshold: f64, max_slow_queries: usize, max_frequent_queries: usize) -> Self {
        Self {
            slow_query_threshold,
            max_slow_queries,
            max_frequent_queries,
        }
    }

    /// Analyze queries from log entries
    pub fn analyze_queries(&self, entries: &[LogEntry]) -> Result<AnalysisResult> {
        let mut result = AnalysisResult::new();
        let mut query_durations = Vec::new();
        let mut query_counts = HashMap::new();

        for entry in entries {
            if entry.is_query() {
                if let Some(query) = &entry.query {
                    // Count query occurrences
                    let normalized = entry.normalized_query().unwrap_or_default();
                    *query_counts.entry(normalized).or_insert(0) += 1;

                    // Add to analysis result
                    let duration = entry.duration.unwrap_or(0.0);
                    result.add_query(query, duration);
                    query_durations.push(duration);
                }
            } else if entry.is_error() {
                result.add_error();
            } else if entry.message.contains("connection") {
                result.add_connection();
            }
        }

        // Calculate percentiles
        result.calculate_percentiles(&query_durations);

        // Find slowest queries
        let mut slow_queries: Vec<_> = entries
            .iter()
            .filter(|e| e.is_query() && e.duration.unwrap_or(0.0) > self.slow_query_threshold)
            .collect();
        slow_queries.sort_by(|a, b| b.duration.unwrap_or(0.0).partial_cmp(&a.duration.unwrap_or(0.0)).unwrap());

        result.slowest_queries = slow_queries
            .into_iter()
            .take(self.max_slow_queries)
            .map(|e| (e.query.clone().unwrap_or_default(), e.duration.unwrap_or(0.0)))
            .collect();

        // Find most frequent queries
        let mut frequent_queries: Vec<_> = query_counts.into_iter().collect();
        frequent_queries.sort_by(|a, b| b.1.cmp(&a.1));

        result.most_frequent_queries = frequent_queries
            .into_iter()
            .take(self.max_frequent_queries)
            .collect();

        Ok(result)
    }

    /// Find slow queries above a threshold
    pub fn find_slow_queries(&self, entries: &[LogEntry], threshold_ms: f64) -> Result<Vec<LogEntry>> {
        let slow_queries: Vec<_> = entries
            .iter()
            .filter(|e| e.is_query() && e.duration.unwrap_or(0.0) > threshold_ms)
            .cloned()
            .collect();

        Ok(slow_queries)
    }
}

impl Default for QueryAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
