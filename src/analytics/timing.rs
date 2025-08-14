//! Performance timing analysis for PostgreSQL logs

use crate::{LogEntry, Result, analytics_error};
use chrono::{Duration, Datelike, Timelike};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Analyzer for timing and performance metrics
pub struct TimingAnalyzer {
    // Configuration for timing analysis
    time_bucket_size: u32, // minutes
}

impl TimingAnalyzer {
    /// Create a new timing analyzer
    pub fn new() -> Self {
        Self {
            time_bucket_size: 60, // 1 hour default
        }
    }

    /// Create a new timing analyzer with custom bucket size
    pub fn with_bucket_size(time_bucket_size: u32) -> Self {
        Self { time_bucket_size }
    }

    /// Analyze timing patterns in log entries
    pub fn analyze_timing(&self, entries: &[LogEntry]) -> Result<TimingAnalysis> {
        let mut hourly_patterns = HashMap::new();
        let mut daily_patterns = HashMap::new();
        let mut response_times = Vec::new();

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
        }

        // Calculate statistics
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

        Ok(TimingAnalysis {
            average_response_time: Duration::milliseconds(avg_response_time as i64),
            p95_response_time: Duration::milliseconds(p95_response_time as i64),
            p99_response_time: Duration::milliseconds(p99_response_time as i64),
            hourly_patterns,
            daily_patterns,
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
}
