//! pg-loggrep - PostgreSQL log analysis tool
//!
//! This library provides tools for parsing and analyzing PostgreSQL log files.
//! It includes robust error handling, comprehensive data structures, and
//! production-ready analysis capabilities.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

pub mod parsers;
pub mod analytics;
pub mod output;

// Re-export commonly used items
pub use parsers::StderrParser;
pub use analytics::{QueryAnalyzer, TimingAnalyzer, TimingAnalysis};
pub use output::{JsonFormatter, TextFormatter};

/// Main error type for pg-loggrep operations
#[derive(Error, Debug)]
pub enum PgLoggrepError {
    /// I/O errors when reading files or writing output
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Errors parsing log files or individual log lines
    #[error("Parse error: {message}")]
    Parse {
        message: String,
        line_number: Option<usize>,
        line_content: Option<String>,
    },

    /// Errors parsing timestamps in log entries
    #[error("Timestamp parse error: {message}")]
    TimestampParse {
        message: String,
        timestamp_string: String,
    },

    /// Configuration errors from CLI arguments or settings
    #[error("Configuration error: {message}")]
    Configuration {
        message: String,
        field: Option<String>,
    },

    /// Errors during analytics computation
    #[error("Analytics error: {message}")]
    Analytics {
        message: String,
        operation: String,
    },

    /// Errors serializing/deserializing data
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Generic error for unexpected conditions
    #[error("Unexpected error: {message}")]
    Unexpected {
        message: String,
        context: Option<String>,
    },
}

/// Log level enumeration for PostgreSQL log entries
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LogLevel {
    /// Error messages
    Error,
    /// Warning messages
    Warning,
    /// Information messages
    Info,
    /// Debug messages
    Debug,
    /// Notice messages
    Notice,
    /// Log messages
    Log,
    /// Statement messages
    Statement,
    /// Duration messages
    Duration,
    /// Unknown or unrecognized log level
    Unknown(String),
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Warning => write!(f, "WARNING"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Notice => write!(f, "NOTICE"),
            LogLevel::Log => write!(f, "LOG"),
            LogLevel::Statement => write!(f, "STATEMENT"),
            LogLevel::Duration => write!(f, "DURATION"),
            LogLevel::Unknown(s) => write!(f, "{}", s.to_uppercase()),
        }
    }
}

impl From<&str> for LogLevel {
    fn from(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "ERROR" => LogLevel::Error,
            "WARNING" => LogLevel::Warning,
            "INFO" => LogLevel::Info,
            "DEBUG" => LogLevel::Debug,
            "NOTICE" => LogLevel::Notice,
            "LOG" => LogLevel::Log,
            "STATEMENT" => LogLevel::Statement,
            "DURATION" => LogLevel::Duration,
            _ => LogLevel::Unknown(s.to_string()),
        }
    }
}

/// Represents a single parsed PostgreSQL log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Timestamp when the log entry was generated
    pub timestamp: DateTime<Utc>,
    /// PostgreSQL process ID
    pub process_id: String,
    /// Database user (if available)
    pub user: Option<String>,
    /// Database name (if available)
    pub database: Option<String>,
    /// Client host address (if available)
    pub client_host: Option<String>,
    /// Application name (if available)
    pub application_name: Option<String>,
    /// Type/level of the log message
    pub message_type: LogLevel,
    /// The main log message content
    pub message: String,
    /// SQL query (if this is a statement log)
    pub query: Option<String>,
    /// Query duration in milliseconds (if available)
    pub duration: Option<f64>,
}

impl LogEntry {
    /// Create a new LogEntry with required fields
    pub fn new(
        timestamp: DateTime<Utc>,
        process_id: String,
        message_type: LogLevel,
        message: String,
    ) -> Self {
        Self {
            timestamp,
            process_id,
            user: None,
            database: None,
            client_host: None,
            application_name: None,
            message_type,
            message,
            query: None,
            duration: None,
        }
    }

    /// Check if this log entry represents a query statement
    pub fn is_query(&self) -> bool {
        matches!(self.message_type, LogLevel::Statement)
    }

    /// Check if this log entry represents a duration measurement
    pub fn is_duration(&self) -> bool {
        matches!(self.message_type, LogLevel::Duration)
    }

    /// Check if this log entry represents an error
    pub fn is_error(&self) -> bool {
        matches!(self.message_type, LogLevel::Error)
    }

    /// Get the normalized query (for deduplication)
    pub fn normalized_query(&self) -> Option<String> {
        self.query.as_ref().map(|q| {
            // Basic normalization - remove extra whitespace and convert to lowercase
            q.trim().to_lowercase()
        })
    }
}

/// Contains aggregated statistics from log analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    /// Total number of queries processed
    pub total_queries: u64,
    /// Total duration of all queries in milliseconds
    pub total_duration: f64,
    /// Count of queries by type (SELECT, INSERT, UPDATE, DELETE, etc.)
    pub query_types: HashMap<String, u64>,
    /// Slowest queries with their durations
    pub slowest_queries: Vec<(String, f64)>,
    /// Most frequent queries with their counts
    pub most_frequent_queries: Vec<(String, u64)>,
    /// Total number of error messages
    pub error_count: u64,
    /// Total number of connection events
    pub connection_count: u64,
    /// Average query duration in milliseconds
    pub average_duration: f64,
    /// 95th percentile query duration in milliseconds
    pub p95_duration: f64,
    /// 99th percentile query duration in milliseconds
    pub p99_duration: f64,
}

impl AnalysisResult {
    /// Create a new empty AnalysisResult
    pub fn new() -> Self {
        Self {
            total_queries: 0,
            total_duration: 0.0,
            query_types: HashMap::new(),
            slowest_queries: Vec::new(),
            most_frequent_queries: Vec::new(),
            error_count: 0,
            connection_count: 0,
            average_duration: 0.0,
            p95_duration: 0.0,
            p99_duration: 0.0,
        }
    }

    /// Add a query to the analysis
    pub fn add_query(&mut self, query: &str, duration: f64) {
        self.total_queries += 1;
        self.total_duration += duration;

        // Update query type count
        let query_type = self.extract_query_type(query);
        *self.query_types.entry(query_type).or_insert(0) += 1;

        // Update average duration
        self.average_duration = self.total_duration / self.total_queries as f64;
    }

    /// Add an error to the count
    pub fn add_error(&mut self) {
        self.error_count += 1;
    }

    /// Add a connection event to the count
    pub fn add_connection(&mut self) {
        self.connection_count += 1;
    }

    /// Extract the query type from a SQL query
    fn extract_query_type(&self, query: &str) -> String {
        let query_upper = query.trim().to_uppercase();
        if query_upper.starts_with("SELECT") {
            "SELECT".to_string()
        } else if query_upper.starts_with("INSERT") {
            "INSERT".to_string()
        } else if query_upper.starts_with("UPDATE") {
            "UPDATE".to_string()
        } else if query_upper.starts_with("DELETE") {
            "DELETE".to_string()
        } else if query_upper.starts_with("CREATE") {
            "CREATE".to_string()
        } else if query_upper.starts_with("DROP") {
            "DROP".to_string()
        } else if query_upper.starts_with("ALTER") {
            "ALTER".to_string()
        } else if query_upper.starts_with("BEGIN") || query_upper.starts_with("COMMIT") || query_upper.starts_with("ROLLBACK") {
            "TRANSACTION".to_string()
        } else {
            "OTHER".to_string()
        }
    }

    /// Calculate percentiles from a list of durations
    pub fn calculate_percentiles(&mut self, durations: &[f64]) {
        if durations.is_empty() {
            return;
        }

        let mut sorted_durations = durations.to_vec();
        sorted_durations.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let len = sorted_durations.len();
        let p95_index = (len as f64 * 0.95) as usize;
        let p99_index = (len as f64 * 0.99) as usize;

        self.p95_duration = sorted_durations[p95_index.min(len - 1)];
        self.p99_duration = sorted_durations[p99_index.min(len - 1)];
    }
}

impl Default for AnalysisResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Result type alias for pg-loggrep operations
pub type Result<T> = std::result::Result<T, PgLoggrepError>;

/// Helper function to create parse errors with context
pub fn parse_error(message: &str, line_number: Option<usize>, line_content: Option<&str>) -> PgLoggrepError {
    PgLoggrepError::Parse {
        message: message.to_string(),
        line_number,
        line_content: line_content.map(|s| s.to_string()),
    }
}

/// Helper function to create timestamp parse errors
pub fn timestamp_error(message: &str, timestamp_string: &str) -> PgLoggrepError {
    PgLoggrepError::TimestampParse {
        message: message.to_string(),
        timestamp_string: timestamp_string.to_string(),
    }
}

/// Helper function to create configuration errors
pub fn config_error(message: &str, field: Option<&str>) -> PgLoggrepError {
    PgLoggrepError::Configuration {
        message: message.to_string(),
        field: field.map(|s| s.to_string()),
    }
}

/// Helper function to create analytics errors
pub fn analytics_error(message: &str, operation: &str) -> PgLoggrepError {
    PgLoggrepError::Analytics {
        message: message.to_string(),
        operation: operation.to_string(),
    }
}
