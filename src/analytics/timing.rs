//! Performance timing analysis for PostgreSQL logs

use crate::{LogEntry, Result, analytics_error};
use chrono::{DateTime, Utc, Datelike, Timelike, Duration};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Timing analyzer configuration
#[derive(Debug, Clone)]
pub struct TimingAnalyzerConfig {
    /// Time bucket size in minutes for aggregation
    pub time_bucket_size: u32,
    /// Whether to include connection analysis
    pub include_connections: bool,
    /// Whether to include peak usage analysis
    pub include_peak_analysis: bool,
}

impl Default for TimingAnalyzerConfig {
    fn default() -> Self {
        Self {
            time_bucket_size: 60, // 1 hour default
            include_connections: true,
            include_peak_analysis: true,
        }
    }
}

/// Analyzer for timing and performance metrics
pub struct TimingAnalyzer {
    /// Configuration for timing analysis
    config: TimingAnalyzerConfig,
}

impl TimingAnalyzer {
    /// Create a new timing analyzer with default configuration
    pub fn new() -> Self {
        Self {
            config: TimingAnalyzerConfig::default(),
        }
    }

    /// Create a new timing analyzer with custom configuration
    pub fn with_config(config: TimingAnalyzerConfig) -> Self {
        Self { config }
    }

    /// Create a new timing analyzer with custom bucket size
    pub fn with_bucket_size(time_bucket_size: u32) -> Self {
        Self {
            config: TimingAnalyzerConfig {
                time_bucket_size,
                ..Default::default()
            },
        }
    }

    /// Analyze timing patterns in log entries
    pub fn analyze_timing(&self, entries: &[LogEntry]) -> Result<TimingAnalysis> {
        if entries.is_empty() {
            return Ok(TimingAnalysis::default());
        }

        let mut hourly_patterns = HashMap::new();
        let mut daily_patterns = HashMap::new();
        let mut response_times = Vec::new();
        let mut connection_patterns = HashMap::new();
        let mut peak_hours = Vec::new();

        // Process each entry
        for entry in entries {
            if let Some(duration) = entry.duration {
                response_times.push(duration);

                // Group by hour
                let hour = entry.timestamp.hour();
                let current_duration = hourly_patterns.entry(hour).or_insert(0.0);
                *current_duration += duration;

                // Group by day of week
                let day = entry.timestamp.weekday().num_days_from_monday();
                let current_day_duration = daily_patterns.entry(day).or_insert(0.0);
                *current_day_duration += duration;
            }

            // Analyze connection patterns if enabled
            if self.config.include_connections && entry.message.to_lowercase().contains("connection") {
                let hour = entry.timestamp.hour();
                *connection_patterns.entry(hour).or_insert(0) += 1;
            }
        }

        // Calculate basic statistics
        let avg_response_time = if !response_times.is_empty() {
            response_times.iter().sum::<f64>() / response_times.len() as f64
        } else {
            0.0
        };

        let mut sorted_times = response_times.clone();
        sorted_times.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let p95_response_time = if !sorted_times.is_empty() {
            let p95_index = (sorted_times.len() as f64 * 0.95) as usize;
            sorted_times[p95_index.min(sorted_times.len() - 1)]
        } else {
            0.0
        };

        let p99_response_time = if !sorted_times.is_empty() {
            let p99_index = (sorted_times.len() as f64 * 0.99) as usize;
            sorted_times[p99_index.min(sorted_times.len() - 1)]
        } else {
            0.0
        };

        // Identify peak usage hours if enabled
        if self.config.include_peak_analysis {
            peak_hours = self.identify_peak_hours(&hourly_patterns);
        }

        Ok(TimingAnalysis {
            average_response_time: Duration::milliseconds(avg_response_time as i64),
            p95_response_time: Duration::milliseconds(p95_response_time as i64),
            p99_response_time: Duration::milliseconds(p99_response_time as i64),
            hourly_patterns,
            daily_patterns,
            connection_patterns,
            peak_hours,
            total_queries: response_times.len() as u64,
            total_duration: response_times.iter().sum(),
        })
    }

    /// Calculate response time percentiles
    pub fn calculate_percentiles(&self, response_times: &[f64], percentiles: &[f64]) -> Result<Vec<(f64, f64)>> {
        if response_times.is_empty() {
            return Err(analytics_error("No response times provided", "calculate_percentiles"));
        }

        let mut sorted_times = response_times.to_vec();
        sorted_times.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let mut result = Vec::new();
        for &percentile in percentiles {
            if percentile < 0.0 || percentile > 1.0 {
                return Err(analytics_error(
                    &format!("Invalid percentile: {}", percentile),
                    "calculate_percentiles"
                ));
            }

            let index = (sorted_times.len() as f64 * percentile) as usize;
            let value = sorted_times[index.min(sorted_times.len() - 1)];
            result.push((percentile, value));
        }

        Ok(result)
    }

    /// Analyze hourly query distribution
    pub fn analyze_hourly_distribution(&self, entries: &[LogEntry]) -> Result<HashMap<u32, HourlyMetrics>> {
        let mut hourly_metrics = HashMap::new();

        for entry in entries {
            if entry.is_query() {
                let hour = entry.timestamp.hour();
                let metrics = hourly_metrics.entry(hour).or_insert_with(|| HourlyMetrics {
                    hour,
                    query_count: 0,
                    total_duration: 0.0,
                    average_duration: 0.0,
                    min_duration: f64::INFINITY,
                    max_duration: 0.0,
                    queries_per_second: 0.0,
                });

                let duration = entry.duration.unwrap_or(0.0);
                metrics.query_count += 1;
                metrics.total_duration += duration;
                metrics.min_duration = metrics.min_duration.min(duration);
                metrics.max_duration = metrics.max_duration.max(duration);
            }
        }

        // Calculate averages and queries per second
        for metrics in hourly_metrics.values_mut() {
            if metrics.query_count > 0 {
                metrics.average_duration = metrics.total_duration / metrics.query_count as f64;
            }
            if metrics.min_duration == f64::INFINITY {
                metrics.min_duration = 0.0;
            }
        }

        // Calculate queries per second (simplified - would need time range for accurate calculation)
        self.calculate_queries_per_second(&mut hourly_metrics, entries);

        Ok(hourly_metrics)
    }

    /// Analyze connection patterns
    pub fn analyze_connection_patterns(&self, entries: &[LogEntry]) -> Result<ConnectionAnalysis> {
        let mut hourly_connections = HashMap::new();
        let mut daily_connections = HashMap::new();
        let mut total_connections = 0;
        let mut connection_errors = 0;

        for entry in entries {
            if entry.message.to_lowercase().contains("connection") {
                total_connections += 1;

                let hour = entry.timestamp.hour();
                *hourly_connections.entry(hour).or_insert(0) += 1;

                let day = entry.timestamp.weekday().num_days_from_monday();
                *daily_connections.entry(day).or_insert(0) += 1;

                if entry.is_error() {
                    connection_errors += 1;
                }
            }
        }

        Ok(ConnectionAnalysis {
            total_connections,
            connection_errors,
            hourly_connections,
            daily_connections,
            error_rate: if total_connections > 0 {
                connection_errors as f64 / total_connections as f64
            } else {
                0.0
            },
        })
    }

    /// Identify peak usage hours
    fn identify_peak_hours(&self, hourly_patterns: &HashMap<u32, f64>) -> Vec<u32> {
        if hourly_patterns.is_empty() {
            return Vec::new();
        }

        let avg_duration = hourly_patterns.values().sum::<f64>() / hourly_patterns.len() as f64;
        let threshold = avg_duration * 1.5; // 50% above average

        let mut peak_hours: Vec<_> = hourly_patterns
            .iter()
            .filter(|(_, &duration)| duration > threshold)
            .map(|(&hour, _)| hour)
            .collect();

        peak_hours.sort();
        peak_hours
    }

    /// Calculate queries per second for hourly buckets
    fn calculate_queries_per_second(&self, hourly_metrics: &mut HashMap<u32, HourlyMetrics>, entries: &[LogEntry]) {
        // Group entries by hour to calculate time spans
        let mut hourly_entries: HashMap<u32, Vec<DateTime<Utc>>> = HashMap::new();

        for entry in entries {
            if entry.is_query() {
                let hour = entry.timestamp.hour();
                hourly_entries.entry(hour).or_default().push(entry.timestamp);
            }
        }

        for (hour, timestamps) in hourly_entries {
            if let Some(metrics) = hourly_metrics.get_mut(&hour) {
                if timestamps.len() > 1 {
                    let min_time = timestamps.iter().min().unwrap();
                    let max_time = timestamps.iter().max().unwrap();
                    let duration_seconds = (*max_time - *min_time).num_seconds() as f64;

                    if duration_seconds > 0.0 {
                        metrics.queries_per_second = metrics.query_count as f64 / duration_seconds;
                    }
                }
            }
        }
    }

    /// Get peak usage analysis
    pub fn get_peak_usage_analysis(&self, entries: &[LogEntry]) -> Result<PeakUsageAnalysis> {
        let hourly_distribution = self.analyze_hourly_distribution(entries)?;

        if hourly_distribution.is_empty() {
            return Ok(PeakUsageAnalysis::default());
        }

        let max_queries = hourly_distribution.values().map(|m| m.query_count).max().unwrap_or(0);
        let max_duration = hourly_distribution.values().map(|m| m.total_duration).fold(0.0_f64, f64::max);

        let peak_hours: Vec<_> = hourly_distribution
            .iter()
            .filter(|(_, metrics)| {
                metrics.query_count as f64 >= max_queries as f64 * 0.8 || // 80% of max queries
                metrics.total_duration >= max_duration * 0.8 // 80% of max duration
            })
            .map(|(&hour, _)| hour)
            .collect();

        let busiest_hour = hourly_distribution
            .iter()
            .max_by(|(_, a), (_, b)| a.query_count.cmp(&b.query_count))
            .map(|(&hour, _)| hour);

        Ok(PeakUsageAnalysis {
            peak_hours,
            busiest_hour,
            max_queries_per_hour: max_queries,
            max_duration_per_hour: max_duration,
            average_queries_per_hour: hourly_distribution.values().map(|m| m.query_count).sum::<u64>() / hourly_distribution.len() as u64,
        })
    }
}

impl Default for TimingAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

/// Results of timing analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingAnalysis {
    pub average_response_time: Duration,
    pub p95_response_time: Duration,
    pub p99_response_time: Duration,
    pub hourly_patterns: HashMap<u32, f64>,
    pub daily_patterns: HashMap<u32, f64>,
    pub connection_patterns: HashMap<u32, u64>,
    pub peak_hours: Vec<u32>,
    pub total_queries: u64,
    pub total_duration: f64,
}

impl Default for TimingAnalysis {
    fn default() -> Self {
        Self {
            average_response_time: Duration::zero(),
            p95_response_time: Duration::zero(),
            p99_response_time: Duration::zero(),
            hourly_patterns: HashMap::new(),
            daily_patterns: HashMap::new(),
            connection_patterns: HashMap::new(),
            peak_hours: Vec::new(),
            total_queries: 0,
            total_duration: 0.0,
        }
    }
}

/// Hourly metrics for detailed analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourlyMetrics {
    pub hour: u32,
    pub query_count: u64,
    pub total_duration: f64,
    pub average_duration: f64,
    pub min_duration: f64,
    pub max_duration: f64,
    pub queries_per_second: f64,
}

/// Connection pattern analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionAnalysis {
    pub total_connections: u64,
    pub connection_errors: u64,
    pub hourly_connections: HashMap<u32, u64>,
    pub daily_connections: HashMap<u32, u64>,
    pub error_rate: f64,
}

/// Peak usage analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeakUsageAnalysis {
    pub peak_hours: Vec<u32>,
    pub busiest_hour: Option<u32>,
    pub max_queries_per_hour: u64,
    pub max_duration_per_hour: f64,
    pub average_queries_per_hour: u64,
}

impl Default for PeakUsageAnalysis {
    fn default() -> Self {
        Self {
            peak_hours: Vec::new(),
            busiest_hour: None,
            max_queries_per_hour: 0,
            max_duration_per_hour: 0.0,
            average_queries_per_hour: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LogLevel;

    fn create_test_entry(
        timestamp: DateTime<Utc>,
        message_type: LogLevel,
        duration: Option<f64>,
        message: &str,
    ) -> LogEntry {
        LogEntry {
            timestamp,
            process_id: "12345".to_string(),
            user: Some("test_user".to_string()),
            database: Some("testdb".to_string()),
            client_host: None,
            application_name: Some("psql".to_string()),
            message_type,
            message: message.to_string(),
            query: None,
            duration,
        }
    }

    #[test]
    fn test_analyze_timing_empty_entries() {
        let analyzer = TimingAnalyzer::new();
        let result = analyzer.analyze_timing(&[]).unwrap();

        assert_eq!(result.total_queries, 0);
        assert_eq!(result.total_duration, 0.0);
        assert!(result.hourly_patterns.is_empty());
    }

    #[test]
    fn test_analyze_timing_with_entries() {
        let analyzer = TimingAnalyzer::new();
        let now = Utc::now();

        let entries = vec![
            create_test_entry(now, LogLevel::Statement, Some(100.0), "statement: SELECT 1"),
            create_test_entry(now, LogLevel::Statement, Some(200.0), "statement: SELECT 2"),
            create_test_entry(now, LogLevel::Statement, Some(300.0), "statement: SELECT 3"),
        ];

        let result = analyzer.analyze_timing(&entries).unwrap();

        assert_eq!(result.total_queries, 3);
        assert_eq!(result.total_duration, 600.0);
        assert_eq!(result.average_response_time.num_milliseconds(), 200);
    }

    #[test]
    fn test_calculate_percentiles() {
        let analyzer = TimingAnalyzer::new();
        let response_times = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0];
        let percentiles = vec![0.5, 0.95, 0.99];

        let result = analyzer.calculate_percentiles(&response_times, &percentiles).unwrap();

        assert_eq!(result.len(), 3);
        assert_eq!(result[0], (0.5, 60.0)); // median (5th element in 0-indexed array)
        assert_eq!(result[1], (0.95, 100.0)); // p95 (9th element in 0-indexed array)
        assert_eq!(result[2], (0.99, 100.0)); // p99 (9th element in 0-indexed array)
    }

    #[test]
    fn test_analyze_connection_patterns() {
        let analyzer = TimingAnalyzer::new();
        let now = Utc::now();

        let entries = vec![
            create_test_entry(now, LogLevel::Log, None, "connection received"),
            create_test_entry(now, LogLevel::Log, None, "connection established"),
            create_test_entry(now, LogLevel::Error, None, "connection failed"),
        ];

        let result = analyzer.analyze_connection_patterns(&entries).unwrap();

        assert_eq!(result.total_connections, 3);
        assert_eq!(result.connection_errors, 1);
        assert_eq!(result.error_rate, 1.0 / 3.0);
    }

    #[test]
    fn test_peak_usage_analysis() {
        let analyzer = TimingAnalyzer::new();
        let now = Utc::now();

        // Create entries with varying query counts per hour
        let mut entries = Vec::new();

        // Hour 10: 5 queries
        for i in 0..5 {
            let timestamp = (now + Duration::hours(10)).with_nanosecond(i * 1_000_000).unwrap();
            entries.push(create_test_entry(timestamp, LogLevel::Statement, Some(100.0), "statement: SELECT 1"));
        }

        // Hour 11: 10 queries (peak)
        for i in 0..10 {
            let timestamp = (now + Duration::hours(11)).with_nanosecond(i * 1_000_000).unwrap();
            entries.push(create_test_entry(timestamp, LogLevel::Statement, Some(100.0), "statement: SELECT 1"));
        }

        // Hour 12: 3 queries
        for i in 0..3 {
            let timestamp = (now + Duration::hours(10)).with_nanosecond(i * 1_000_000).unwrap();
            entries.push(create_test_entry(timestamp, LogLevel::Statement, Some(100.0), "statement: SELECT 1"));
        }

        let result = analyzer.get_peak_usage_analysis(&entries).unwrap();

        assert_eq!(result.max_queries_per_hour, 10);
        // The busiest hour depends on the current time, so we'll just check it's one of the expected hours
        assert!(result.busiest_hour.is_some());
        assert!(result.peak_hours.len() > 0);
    }

    #[test]
    fn test_invalid_percentile() {
        let analyzer = TimingAnalyzer::new();
        let response_times = vec![10.0, 20.0, 30.0];
        let percentiles = vec![1.5]; // Invalid percentile > 1.0

        let result = analyzer.calculate_percentiles(&response_times, &percentiles);
        assert!(result.is_err());
    }
}
