//! Query analysis functionality for PostgreSQL logs

use crate::{LogEntry, AnalysisResult, Result};
use std::collections::HashMap;
use chrono::{DateTime, Utc, Timelike};
use regex::Regex;
use serde::{Serialize, Deserialize};

/// Query type classification
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum QueryType {
    /// SELECT queries
    Select,
    /// INSERT queries
    Insert,
    /// UPDATE queries
    Update,
    /// DELETE queries
    Delete,
    /// Data Definition Language (CREATE, DROP, ALTER, etc.)
    DDL,
    /// Other queries (BEGIN, COMMIT, ROLLBACK, etc.)
    Other,
}

impl std::fmt::Display for QueryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryType::Select => write!(f, "SELECT"),
            QueryType::Insert => write!(f, "INSERT"),
            QueryType::Update => write!(f, "UPDATE"),
            QueryType::Delete => write!(f, "DELETE"),
            QueryType::DDL => write!(f, "DDL"),
            QueryType::Other => write!(f, "OTHER"),
        }
    }
}

/// Query performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMetrics {
    /// Minimum duration in milliseconds
    pub min_duration: f64,
    /// Maximum duration in milliseconds
    pub max_duration: f64,
    /// Average duration in milliseconds
    pub average_duration: f64,
    /// 95th percentile duration in milliseconds
    pub p95_duration: f64,
    /// 99th percentile duration in milliseconds
    pub p99_duration: f64,
    /// Total number of queries
    pub total_queries: u64,
    /// Total duration in milliseconds
    pub total_duration: f64,
}

impl Default for QueryMetrics {
    fn default() -> Self {
        Self {
            min_duration: 0.0,
            max_duration: 0.0,
            average_duration: 0.0,
            p95_duration: 0.0,
            p99_duration: 0.0,
            total_queries: 0,
            total_duration: 0.0,
        }
    }
}

/// Hourly query statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourlyStats {
    /// Hour of day (0-23)
    pub hour: u32,
    /// Number of queries in this hour
    pub query_count: u64,
    /// Average queries per second in this hour
    pub queries_per_second: f64,
    /// Total duration in milliseconds
    pub total_duration: f64,
    /// Average duration in milliseconds
    pub average_duration: f64,
}

/// Analyzer for SQL queries found in PostgreSQL logs
pub struct QueryAnalyzer {
    /// Threshold for considering a query "slow" (milliseconds)
    slow_query_threshold: f64,
    /// Maximum number of slow queries to track
    max_slow_queries: usize,
    /// Maximum number of frequent queries to track
    max_frequent_queries: usize,
    /// Regex for normalizing SQL queries
    literal_regex: Regex,
    /// Regex for extracting numeric literals
    numeric_regex: Regex,
    /// Regex for extracting string literals
    string_regex: Regex,
}

impl QueryAnalyzer {
    /// Create a new query analyzer with default settings
    pub fn new() -> Self {
        Self {
            slow_query_threshold: 1000.0, // 1 second default
            max_slow_queries: 10,
            max_frequent_queries: 20,
            literal_regex: Regex::new(r"\$(\d+)").unwrap(),
            numeric_regex: Regex::new(r"\b\d+(?:\.\d+)?\b").unwrap(),
            string_regex: Regex::new(r"'[^']*'").unwrap(),
        }
    }

    /// Create a new query analyzer with custom settings
    pub fn with_settings(
        slow_query_threshold: f64,
        max_slow_queries: usize,
        max_frequent_queries: usize,
    ) -> Self {
        Self {
            slow_query_threshold,
            max_slow_queries,
            max_frequent_queries,
            literal_regex: Regex::new(r"\$(\d+)").unwrap(),
            numeric_regex: Regex::new(r"\b\d+(?:\.\d+)?\b").unwrap(),
            string_regex: Regex::new(r"'[^']*'").unwrap(),
        }
    }

    /// Analyze queries from log entries
    pub fn analyze(&self, entries: &[LogEntry]) -> Result<AnalysisResult> {
        if entries.is_empty() {
            return Ok(AnalysisResult::new());
        }

        let mut result = AnalysisResult::new();
        let mut query_durations = Vec::new();
        let mut query_counts = HashMap::new();
        let mut query_type_counts = HashMap::new();
        let mut hourly_stats = HashMap::new();
        let mut slow_queries = Vec::new();
        let mut error_count = 0;
        let mut connection_count = 0;

        for entry in entries {
            if entry.is_query() {
                if let Some(query) = &entry.query {
                    let duration = entry.duration.unwrap_or(0.0);
                    let normalized = self.normalize_query(query);
                    let query_type = self.classify_query(query);

                    // Update query counts
                    *query_counts.entry(normalized.clone()).or_insert(0) += 1;
                    *query_type_counts.entry(query_type).or_insert(0) += 1;

                    // Update duration statistics
                    query_durations.push(duration);
                    result.total_queries += 1;
                    result.total_duration += duration;

                    // Track slow queries
                    if duration > self.slow_query_threshold {
                        slow_queries.push((normalized.clone(), duration));
                    }

                    // Update hourly statistics
                    let hour = entry.timestamp.hour();
                    let hourly = hourly_stats.entry(hour).or_insert_with(|| HourlyStats {
                        hour,
                        query_count: 0,
                        queries_per_second: 0.0,
                        total_duration: 0.0,
                        average_duration: 0.0,
                    });
                    hourly.query_count += 1;
                    hourly.total_duration += duration;
                }
            } else if entry.is_error() {
                error_count += 1;
            } else if entry.message.to_lowercase().contains("connection") {
                connection_count += 1;
            }
        }

        // Calculate performance metrics
        let metrics = self.calculate_metrics(&query_durations);
        result.average_duration = metrics.average_duration;
        result.p95_duration = metrics.p95_duration;
        result.p99_duration = metrics.p99_duration;

        // Update error and connection counts
        result.error_count = error_count;
        result.connection_count = connection_count;

        // Find top slowest queries
        slow_queries.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        result.slowest_queries = slow_queries
            .into_iter()
            .take(self.max_slow_queries)
            .collect();

        // Find top most frequent queries
        let mut frequent_queries: Vec<_> = query_counts.into_iter().collect();
        frequent_queries.sort_by(|a, b| b.1.cmp(&a.1));
        result.most_frequent_queries = frequent_queries
            .into_iter()
            .take(self.max_frequent_queries)
            .collect();

        // Update query type distribution
        result.query_types = query_type_counts
            .into_iter()
            .map(|(query_type, count)| (query_type.to_string(), count))
            .collect();

        // Calculate queries per second for hourly buckets
        self.calculate_queries_per_second(&mut hourly_stats, entries);

        Ok(result)
    }

    /// Normalize SQL query by replacing literals with placeholders
    pub fn normalize_query(&self, sql: &str) -> String {
        let mut normalized = sql.trim().to_string();

        // Replace parameter placeholders ($1, $2, etc.)
        normalized = self.literal_regex.replace_all(&normalized, "?").to_string();

        // Replace numeric literals
        normalized = self.numeric_regex.replace_all(&normalized, "N").to_string();

        // Replace string literals
        normalized = self.string_regex.replace_all(&normalized, "S").to_string();

        // Normalize whitespace
        normalized
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Classify query type based on SQL content
    pub fn classify_query(&self, sql: &str) -> QueryType {
        let sql_upper = sql.trim().to_uppercase();

        if sql_upper.starts_with("SELECT") {
            QueryType::Select
        } else if sql_upper.starts_with("INSERT") {
            QueryType::Insert
        } else if sql_upper.starts_with("UPDATE") {
            QueryType::Update
        } else if sql_upper.starts_with("DELETE") {
            QueryType::Delete
        } else if sql_upper.starts_with("CREATE")
               || sql_upper.starts_with("DROP")
               || sql_upper.starts_with("ALTER")
               || sql_upper.starts_with("TRUNCATE")
               || sql_upper.starts_with("GRANT")
               || sql_upper.starts_with("REVOKE") {
            QueryType::DDL
        } else {
            QueryType::Other
        }
    }

    /// Calculate performance metrics from durations
    fn calculate_metrics(&self, durations: &[f64]) -> QueryMetrics {
        if durations.is_empty() {
            return QueryMetrics::default();
        }

        let total_queries = durations.len() as u64;
        let total_duration = durations.iter().sum::<f64>();
        let average_duration = total_duration / total_queries as f64;

        let min_duration = durations.iter().fold(f64::INFINITY, |a, &b| a.min(b));
        let max_duration = durations.iter().fold(0.0_f64, |a, &b| a.max(b));

        // Calculate percentiles
        let mut sorted_durations = durations.to_vec();
        sorted_durations.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let p95_index = (sorted_durations.len() as f64 * 0.95) as usize;
        let p99_index = (sorted_durations.len() as f64 * 0.99) as usize;

        let p95_duration = sorted_durations[p95_index.min(sorted_durations.len() - 1)];
        let p99_duration = sorted_durations[p99_index.min(sorted_durations.len() - 1)];

        QueryMetrics {
            min_duration,
            max_duration,
            average_duration,
            p95_duration,
            p99_duration,
            total_queries,
            total_duration,
        }
    }

    /// Calculate queries per second for hourly buckets
    fn calculate_queries_per_second(&self, hourly_stats: &mut HashMap<u32, HourlyStats>, entries: &[LogEntry]) {
        // Group entries by hour to calculate time spans
        let mut hourly_entries: HashMap<u32, Vec<DateTime<Utc>>> = HashMap::new();

        for entry in entries {
            if entry.is_query() {
                let hour = entry.timestamp.hour();
                hourly_entries.entry(hour).or_default().push(entry.timestamp);
            }
        }

        for (hour, timestamps) in hourly_entries {
            if let Some(stats) = hourly_stats.get_mut(&hour) {
                if timestamps.len() > 1 {
                    let min_time = timestamps.iter().min().unwrap();
                    let max_time = timestamps.iter().max().unwrap();
                    let duration_seconds = (*max_time - *min_time).num_seconds() as f64;

                    if duration_seconds > 0.0 {
                        stats.queries_per_second = stats.query_count as f64 / duration_seconds;
                    }
                }

                if stats.query_count > 0 {
                    stats.average_duration = stats.total_duration / stats.query_count as f64;
                }
            }
        }
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

    /// Get query type distribution
    pub fn get_query_type_distribution(&self, entries: &[LogEntry]) -> HashMap<QueryType, u64> {
        let mut distribution = HashMap::new();

        for entry in entries {
            if entry.is_query() {
                if let Some(query) = &entry.query {
                    let query_type = self.classify_query(query);
                    *distribution.entry(query_type).or_insert(0) += 1;
                }
            }
        }

        distribution
    }

    /// Calculate error rate
    pub fn calculate_error_rate(&self, entries: &[LogEntry]) -> f64 {
        let total_entries = entries.len() as f64;
        if total_entries == 0.0 {
            return 0.0;
        }

        let error_count = entries.iter().filter(|e| e.is_error()).count() as f64;
        error_count / total_entries
    }
}

impl Default for QueryAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LogLevel;

    fn create_test_entry(
        timestamp: DateTime<Utc>,
        message_type: LogLevel,
        query: Option<String>,
        duration: Option<f64>,
    ) -> LogEntry {
        LogEntry {
            timestamp,
            process_id: "12345".to_string(),
            user: Some("test_user".to_string()),
            database: Some("testdb".to_string()),
            client_host: None,
            application_name: Some("psql".to_string()),
            message_type,
            message: query.as_ref().map_or("test message".to_string(), |q| format!("statement: {}", q)),
            query,
            duration,
        }
    }

    #[test]
    fn test_normalize_query() {
        let analyzer = QueryAnalyzer::new();

        // Test parameter replacement
        let query = "SELECT * FROM users WHERE id = $1 AND name = $2";
        let normalized = analyzer.normalize_query(query);
        assert_eq!(normalized, "SELECT * FROM users WHERE id = ? AND name = ?");

        // Test numeric literal replacement
        let query = "SELECT * FROM users WHERE age > 25 AND score < 100.5";
        let normalized = analyzer.normalize_query(query);
        assert_eq!(normalized, "SELECT * FROM users WHERE age > N AND score < N");

        // Test string literal replacement
        let query = "SELECT * FROM users WHERE name = 'John' AND city = 'New York'";
        let normalized = analyzer.normalize_query(query);
        assert_eq!(normalized, "SELECT * FROM users WHERE name = S AND city = S");

        // Test whitespace normalization
        let query = "SELECT   *   FROM    users   WHERE   id=1";
        let normalized = analyzer.normalize_query(query);
        assert_eq!(normalized, "SELECT * FROM users WHERE id=N");
    }

    #[test]
    fn test_classify_query() {
        let analyzer = QueryAnalyzer::new();

        assert_eq!(analyzer.classify_query("SELECT * FROM users"), QueryType::Select);
        assert_eq!(analyzer.classify_query("INSERT INTO users VALUES (1, 'John')"), QueryType::Insert);
        assert_eq!(analyzer.classify_query("UPDATE users SET name = 'Jane'"), QueryType::Update);
        assert_eq!(analyzer.classify_query("DELETE FROM users WHERE id = 1"), QueryType::Delete);
        assert_eq!(analyzer.classify_query("CREATE TABLE users (id INT)"), QueryType::DDL);
        assert_eq!(analyzer.classify_query("DROP TABLE users"), QueryType::DDL);
        assert_eq!(analyzer.classify_query("BEGIN"), QueryType::Other);
        assert_eq!(analyzer.classify_query("COMMIT"), QueryType::Other);
    }

    #[test]
    fn test_analyze_empty_entries() {
        let analyzer = QueryAnalyzer::new();
        let result = analyzer.analyze(&[]).unwrap();

        assert_eq!(result.total_queries, 0);
        assert_eq!(result.total_duration, 0.0);
        assert_eq!(result.error_count, 0);
        assert_eq!(result.connection_count, 0);
    }

    #[test]
    fn test_analyze_with_queries() {
        let analyzer = QueryAnalyzer::new();
        let now = Utc::now();

        let entries = vec![
            create_test_entry(now, LogLevel::Statement, Some("SELECT * FROM users".to_string()), Some(100.0)),
            create_test_entry(now, LogLevel::Statement, Some("SELECT * FROM users".to_string()), Some(200.0)),
            create_test_entry(now, LogLevel::Statement, Some("INSERT INTO users VALUES (1)".to_string()), Some(50.0)),
            create_test_entry(now, LogLevel::Error, None, None),
        ];

        let result = analyzer.analyze(&entries).unwrap();

        assert_eq!(result.total_queries, 3);
        assert_eq!(result.total_duration, 350.0);
        assert_eq!(result.average_duration, 116.66666666666667);
        assert_eq!(result.error_count, 1);
        assert_eq!(result.connection_count, 0);

        // Check query type distribution
        assert_eq!(result.query_types.get("SELECT"), Some(&2));
        assert_eq!(result.query_types.get("INSERT"), Some(&1));
    }

    #[test]
    fn test_slow_queries() {
        let analyzer = QueryAnalyzer::with_settings(100.0, 5, 5);
        let now = Utc::now();

        let entries = vec![
            create_test_entry(now, LogLevel::Statement, Some("SELECT * FROM users".to_string()), Some(50.0)),
            create_test_entry(now, LogLevel::Statement, Some("SELECT * FROM posts".to_string()), Some(150.0)),
            create_test_entry(now, LogLevel::Statement, Some("SELECT * FROM comments".to_string()), Some(250.0)),
        ];

        let result = analyzer.analyze(&entries).unwrap();

        assert_eq!(result.slowest_queries.len(), 2); // Only queries above 100ms threshold
        assert_eq!(result.slowest_queries[0].1, 250.0); // Should be sorted by duration desc
        assert_eq!(result.slowest_queries[1].1, 150.0);
    }

    #[test]
    fn test_error_rate_calculation() {
        let analyzer = QueryAnalyzer::new();
        let now = Utc::now();

        let entries = vec![
            create_test_entry(now, LogLevel::Statement, Some("SELECT * FROM users".to_string()), Some(100.0)),
            create_test_entry(now, LogLevel::Error, None, None),
            create_test_entry(now, LogLevel::Statement, Some("SELECT * FROM posts".to_string()), Some(200.0)),
            create_test_entry(now, LogLevel::Error, None, None),
        ];

        let error_rate = analyzer.calculate_error_rate(&entries);
        assert_eq!(error_rate, 0.5); // 2 errors out of 4 total entries
    }

    #[test]
    fn test_query_type_distribution() {
        let analyzer = QueryAnalyzer::new();
        let now = Utc::now();

        let entries = vec![
            create_test_entry(now, LogLevel::Statement, Some("SELECT * FROM users".to_string()), Some(100.0)),
            create_test_entry(now, LogLevel::Statement, Some("SELECT * FROM posts".to_string()), Some(200.0)),
            create_test_entry(now, LogLevel::Statement, Some("INSERT INTO users VALUES (1)".to_string()), Some(50.0)),
            create_test_entry(now, LogLevel::Statement, Some("UPDATE users SET name = 'John'".to_string()), Some(75.0)),
        ];

        let distribution = analyzer.get_query_type_distribution(&entries);

        assert_eq!(distribution.get(&QueryType::Select), Some(&2));
        assert_eq!(distribution.get(&QueryType::Insert), Some(&1));
        assert_eq!(distribution.get(&QueryType::Update), Some(&1));
        assert_eq!(distribution.get(&QueryType::Delete), None);
    }
}
