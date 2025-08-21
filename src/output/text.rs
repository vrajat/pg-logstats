//! Human-readable text output formatter for pg-logstats results

use crate::{AnalysisResult, LogEntry, PgLogstatsError, Result, TimingAnalysis};
use std::fmt::Write;

/// ANSI color helpers (basic)
pub fn bold(s: &str, color: Option<&str>, enable_color: bool) -> String {
    if !enable_color {
        return s.to_string();
    }
    let code = match color.unwrap_or("white") {
        "red" => "\x1b[31;1m",
        "green" => "\x1b[32;1m",
        "yellow" => "\x1b[33;1m",
        "blue" => "\x1b[34;1m",
        "magenta" => "\x1b[35;1m",
        "cyan" => "\x1b[36;1m",
        _ => "\x1b[37;1m",
    };
    format!("{}{}\x1b[0m", code, s)
}

/// Text formatter for analysis results
pub struct TextFormatter {
    // Configuration for text formatting
    enable_color: bool,
}

impl TextFormatter {
    /// Create a new text formatter
    pub fn new() -> Self {
        Self {
            enable_color: false,
        }
    }

    /// Enable or disable ANSI color output
    pub fn with_color(mut self, enable: bool) -> Self {
        self.enable_color = enable;
        self
    }

    /// Get whether color output is enabled
    pub fn is_color_enabled(&self) -> bool {
        self.enable_color
    }

    /// Format query analysis results as text
    pub fn format_query_analysis(&self, analysis: &AnalysisResult) -> Result<String> {
        let mut output = String::new();

        writeln!(
            output,
            "{}",
            bold("Query Analysis Report", Some("cyan"), self.enable_color)
        )
        .map_err(|e| PgLogstatsError::Unexpected {
            message: e.to_string(),
            context: Some("text formatting".to_string()),
        })?;
        writeln!(
            output,
            "{}",
            bold("===================", Some("cyan"), self.enable_color)
        )
        .map_err(|e| PgLogstatsError::Unexpected {
            message: e.to_string(),
            context: Some("text formatting".to_string()),
        })?;
        writeln!(output, "Total Queries: {}", analysis.total_queries).map_err(|e| {
            PgLogstatsError::Unexpected {
                message: e.to_string(),
                context: Some("text formatting".to_string()),
            }
        })?;
        writeln!(output, "Total Duration: {:.2} ms", analysis.total_duration).map_err(|e| {
            PgLogstatsError::Unexpected {
                message: e.to_string(),
                context: Some("text formatting".to_string()),
            }
        })?;
        writeln!(
            output,
            "Average Duration: {:.2} ms",
            analysis.average_duration
        )
        .map_err(|e| PgLogstatsError::Unexpected {
            message: e.to_string(),
            context: Some("text formatting".to_string()),
        })?;
        writeln!(output, "P95 Duration: {:.2} ms", analysis.p95_duration).map_err(|e| {
            PgLogstatsError::Unexpected {
                message: e.to_string(),
                context: Some("text formatting".to_string()),
            }
        })?;
        writeln!(output, "P99 Duration: {:.2} ms", analysis.p99_duration).map_err(|e| {
            PgLogstatsError::Unexpected {
                message: e.to_string(),
                context: Some("text formatting".to_string()),
            }
        })?;
        writeln!(output, "Error Count: {}", analysis.error_count).map_err(|e| {
            PgLogstatsError::Unexpected {
                message: e.to_string(),
                context: Some("text formatting".to_string()),
            }
        })?;
        writeln!(output, "Connection Count: {}", analysis.connection_count).map_err(|e| {
            PgLogstatsError::Unexpected {
                message: e.to_string(),
                context: Some("text formatting".to_string()),
            }
        })?;

        if !analysis.query_types.is_empty() {
            writeln!(
                output,
                "\n{}",
                bold("Query Types:", Some("yellow"), self.enable_color)
            )
            .map_err(|e| PgLogstatsError::Unexpected {
                message: e.to_string(),
                context: Some("text formatting".to_string()),
            })?;
            for (query_type, count) in &analysis.query_types {
                writeln!(output, "  {:>8}: {}", query_type, count).map_err(|e| {
                    PgLogstatsError::Unexpected {
                        message: e.to_string(),
                        context: Some("text formatting".to_string()),
                    }
                })?;
            }
        }

        if !analysis.slowest_queries.is_empty() {
            writeln!(
                output,
                "\n{}",
                bold("Slowest Queries:", Some("red"), self.enable_color)
            )
            .map_err(|e| PgLogstatsError::Unexpected {
                message: e.to_string(),
                context: Some("text formatting".to_string()),
            })?;
            writeln!(output, "  {:>4}  {:>12}  {}", "#", "Duration (ms)", "Query").map_err(
                |e| PgLogstatsError::Unexpected {
                    message: e.to_string(),
                    context: Some("text formatting".to_string()),
                },
            )?;
            for (i, (query, duration)) in analysis.slowest_queries.iter().enumerate() {
                writeln!(output, "  {:>4}  {:>12.2}  {}", i + 1, duration, query).map_err(|e| {
                    PgLogstatsError::Unexpected {
                        message: e.to_string(),
                        context: Some("text formatting".to_string()),
                    }
                })?;
            }
        }

        if !analysis.most_frequent_queries.is_empty() {
            writeln!(
                output,
                "\n{}",
                bold("Most Frequent Queries:", Some("green"), self.enable_color)
            )
            .map_err(|e| PgLogstatsError::Unexpected {
                message: e.to_string(),
                context: Some("text formatting".to_string()),
            })?;
            writeln!(output, "  {:>4}  {:>8}  {}", "#", "Count", "Query").map_err(|e| {
                PgLogstatsError::Unexpected {
                    message: e.to_string(),
                    context: Some("text formatting".to_string()),
                }
            })?;
            for (i, (query, count)) in analysis.most_frequent_queries.iter().enumerate() {
                writeln!(output, "  {:>4}  {:>8}  {}", i + 1, count, query).map_err(|e| {
                    PgLogstatsError::Unexpected {
                        message: e.to_string(),
                        context: Some("text formatting".to_string()),
                    }
                })?;
            }
        }

        Ok(output)
    }

    /// Format timing analysis results as text
    pub fn format_timing_analysis(&self, analysis: &TimingAnalysis) -> Result<String> {
        let mut output = String::new();

        writeln!(
            output,
            "{}",
            bold("Timing Analysis Report", Some("cyan"), self.enable_color)
        )
        .map_err(|e| PgLogstatsError::Unexpected {
            message: e.to_string(),
            context: Some("text formatting".to_string()),
        })?;
        writeln!(
            output,
            "{}",
            bold("====================", Some("cyan"), self.enable_color)
        )
        .map_err(|e| PgLogstatsError::Unexpected {
            message: e.to_string(),
            context: Some("text formatting".to_string()),
        })?;
        writeln!(
            output,
            "Average Response Time: {}ms",
            analysis.average_response_time.num_milliseconds()
        )
        .map_err(|e| PgLogstatsError::Unexpected {
            message: e.to_string(),
            context: Some("text formatting".to_string()),
        })?;
        writeln!(
            output,
            "95th Percentile: {}ms",
            analysis.p95_response_time.num_milliseconds()
        )
        .map_err(|e| PgLogstatsError::Unexpected {
            message: e.to_string(),
            context: Some("text formatting".to_string()),
        })?;
        writeln!(
            output,
            "99th Percentile: {}ms",
            analysis.p99_response_time.num_milliseconds()
        )
        .map_err(|e| PgLogstatsError::Unexpected {
            message: e.to_string(),
            context: Some("text formatting".to_string()),
        })?;

        Ok(output)
    }

    /// Format log entries as text
    pub fn format_log_entries(&self, entries: &[LogEntry]) -> Result<String> {
        let mut output = String::new();

        writeln!(
            output,
            "{}",
            bold(
                &format!("Log Entries ({} total)", entries.len()),
                Some("magenta"),
                self.enable_color
            )
        )
        .map_err(|e| PgLogstatsError::Unexpected {
            message: e.to_string(),
            context: Some("text formatting".to_string()),
        })?;
        writeln!(
            output,
            "{}",
            bold("================", Some("magenta"), self.enable_color)
        )
        .map_err(|e| PgLogstatsError::Unexpected {
            message: e.to_string(),
            context: Some("text formatting".to_string()),
        })?;

        for (i, entry) in entries.iter().enumerate() {
            writeln!(
                output,
                "[{}] {} {}: {}",
                i + 1,
                entry.timestamp.format("%Y-%m-%d %H:%M:%S"),
                entry.message_type,
                entry.message
            )
            .map_err(|e| PgLogstatsError::Unexpected {
                message: e.to_string(),
                context: Some("text formatting".to_string()),
            })?;
        }

        Ok(output)
    }
}

impl Default for TextFormatter {
    fn default() -> Self {
        Self::new()
    }
}
