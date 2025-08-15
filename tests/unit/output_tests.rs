//! Unit tests for output formatters
//!
//! Tests text and JSON output formatting with various edge cases

use pg_loggrep::output::text::TextFormatter;
use pg_loggrep::output::json::JsonFormatter;
use pg_loggrep::{AnalysisResult, TimingAnalysis, LogEntry, LogLevel};
use chrono::{DateTime, Utc, TimeZone, Duration};
use std::collections::HashMap;

/// Helper function to create a test AnalysisResult
fn create_test_analysis_result() -> AnalysisResult {
    let mut query_types = HashMap::new();
    query_types.insert("SELECT".to_string(), 5);
    query_types.insert("INSERT".to_string(), 3);
    query_types.insert("UPDATE".to_string(), 2);
    query_types.insert("DELETE".to_string(), 1);

    let slowest_queries = vec![
        ("SELECT * FROM large_table WHERE complex_condition = ?".to_string(), 2500.0),
        ("UPDATE users SET last_login = NOW() WHERE id = ?".to_string(), 1200.0),
        ("INSERT INTO audit_log (action, timestamp) VALUES (?, ?)".to_string(), 800.0),
    ];

    let most_frequent_queries = vec![
        ("SELECT * FROM users WHERE active = ?".to_string(), 15),
        ("SELECT COUNT(*) FROM orders".to_string(), 8),
        ("INSERT INTO sessions (user_id, token) VALUES (?, ?)".to_string(), 6),
        ("UPDATE users SET last_seen = NOW() WHERE id = ?".to_string(), 4),
    ];

    AnalysisResult {
        total_queries: 11,
        total_duration: 5500.0,
        average_duration: 500.0,
        p95_duration: 2000.0,
        p99_duration: 2400.0,
        error_count: 2,
        connection_count: 3,
        query_types,
        slowest_queries,
        most_frequent_queries,
    }
}

/// Helper function to create a test TimingAnalysis
fn create_test_timing_analysis() -> TimingAnalysis {
    let mut hourly_patterns = HashMap::new();
    hourly_patterns.insert(9, 1200.0);
    hourly_patterns.insert(10, 2500.0);
    hourly_patterns.insert(11, 1800.0);
    hourly_patterns.insert(14, 3200.0);
    hourly_patterns.insert(15, 2100.0);

    TimingAnalysis {
        average_response_time: Duration::milliseconds(450),
        p95_response_time: Duration::milliseconds(1800),
        p99_response_time: Duration::milliseconds(2300),
        hourly_patterns,
    }
}

/// Helper function to create test log entries
fn create_test_log_entries() -> Vec<LogEntry> {
    let base_time = Utc.with_ymd_and_hms(2024, 8, 15, 10, 30, 0).unwrap();

    vec![
        LogEntry {
            timestamp: base_time,
            process_id: "12345".to_string(),
            user: Some("postgres".to_string()),
            database: Some("testdb".to_string()),
            client_host: None,
            application_name: Some("psql".to_string()),
            message_type: LogLevel::Statement,
            message: "statement: SELECT * FROM users WHERE active = true".to_string(),
            query: Some("SELECT * FROM users WHERE active = true".to_string()),
            duration: Some(150.0),
        },
        LogEntry {
            timestamp: base_time + Duration::seconds(1),
            process_id: "12346".to_string(),
            user: Some("admin".to_string()),
            database: Some("analytics".to_string()),
            client_host: Some("192.168.1.100".to_string()),
            application_name: Some("pgbench".to_string()),
            message_type: LogLevel::Error,
            message: "relation \"missing_table\" does not exist".to_string(),
            query: None,
            duration: None,
        },
        LogEntry {
            timestamp: base_time + Duration::seconds(2),
            process_id: "12347".to_string(),
            user: Some("app_user".to_string()),
            database: Some("app_db".to_string()),
            client_host: None,
            application_name: Some("web_app".to_string()),
            message_type: LogLevel::Duration,
            message: "duration: 45.123 ms".to_string(),
            query: None,
            duration: Some(45.123),
        },
    ]
}

#[cfg(test)]
mod text_formatter_tests {
    use super::*;

    #[test]
    fn test_text_formatter_new() {
        let formatter = TextFormatter::new();
        assert!(!formatter.enable_color); // Default should be no color
    }

    #[test]
    fn test_text_formatter_with_color() {
        let formatter = TextFormatter::new().with_color(true);
        assert!(formatter.enable_color);

        let formatter = TextFormatter::new().with_color(false);
        assert!(!formatter.enable_color);
    }

    #[test]
    fn test_format_query_analysis_basic() {
        let formatter = TextFormatter::new();
        let analysis = create_test_analysis_result();

        let result = formatter.format_query_analysis(&analysis);
        assert!(result.is_ok());

        let output = result.unwrap();

        // Check that basic statistics are included
        assert!(output.contains("Total Queries: 11"));
        assert!(output.contains("Total Duration: 5500.00 ms"));
        assert!(output.contains("Average Duration: 500.00 ms"));
        assert!(output.contains("P95 Duration: 2000.00 ms"));
        assert!(output.contains("P99 Duration: 2400.00 ms"));
        assert!(output.contains("Error Count: 2"));
        assert!(output.contains("Connection Count: 3"));
    }

    #[test]
    fn test_format_query_analysis_query_types() {
        let formatter = TextFormatter::new();
        let analysis = create_test_analysis_result();

        let result = formatter.format_query_analysis(&analysis);
        assert!(result.is_ok());

        let output = result.unwrap();

        // Check query types section
        assert!(output.contains("Query Types:"));
        assert!(output.contains("SELECT: 5"));
        assert!(output.contains("INSERT: 3"));
        assert!(output.contains("UPDATE: 2"));
        assert!(output.contains("DELETE: 1"));
    }

    #[test]
    fn test_format_query_analysis_slowest_queries() {
        let formatter = TextFormatter::new();
        let analysis = create_test_analysis_result();

        let result = formatter.format_query_analysis(&analysis);
        assert!(result.is_ok());

        let output = result.unwrap();

        // Check slowest queries section
        assert!(output.contains("Slowest Queries:"));
        assert!(output.contains("Duration (ms)"));
        assert!(output.contains("2500.00"));
        assert!(output.contains("1200.00"));
        assert!(output.contains("800.00"));
        assert!(output.contains("SELECT * FROM large_table"));
        assert!(output.contains("UPDATE users SET last_login"));
    }

    #[test]
    fn test_format_query_analysis_frequent_queries() {
        let formatter = TextFormatter::new();
        let analysis = create_test_analysis_result();

        let result = formatter.format_query_analysis(&analysis);
        assert!(result.is_ok());

        let output = result.unwrap();

        // Check most frequent queries section
        assert!(output.contains("Most Frequent Queries:"));
        assert!(output.contains("Count"));
        assert!(output.contains("15"));
        assert!(output.contains("8"));
        assert!(output.contains("6"));
        assert!(output.contains("4"));
        assert!(output.contains("SELECT * FROM users WHERE active"));
        assert!(output.contains("SELECT COUNT(*) FROM orders"));
    }

    #[test]
    fn test_format_query_analysis_with_color() {
        let formatter = TextFormatter::new().with_color(true);
        let analysis = create_test_analysis_result();

        let result = formatter.format_query_analysis(&analysis);
        assert!(result.is_ok());

        let output = result.unwrap();

        // Should contain ANSI color codes
        assert!(output.contains("\x1b["));
        assert!(output.contains("\x1b[0m")); // Reset code
    }

    #[test]
    fn test_format_query_analysis_empty() {
        let formatter = TextFormatter::new();
        let analysis = AnalysisResult::new();

        let result = formatter.format_query_analysis(&analysis);
        assert!(result.is_ok());

        let output = result.unwrap();

        // Should handle empty analysis gracefully
        assert!(output.contains("Total Queries: 0"));
        assert!(output.contains("Total Duration: 0.00 ms"));
        assert!(output.contains("Average Duration: 0.00 ms"));
        assert!(output.contains("Error Count: 0"));
        assert!(output.contains("Connection Count: 0"));

        // Should not contain sections for empty data
        assert!(!output.contains("Query Types:"));
        assert!(!output.contains("Slowest Queries:"));
        assert!(!output.contains("Most Frequent Queries:"));
    }

    #[test]
    fn test_format_timing_analysis() {
        let formatter = TextFormatter::new();
        let timing = create_test_timing_analysis();

        let result = formatter.format_timing_analysis(&timing);
        assert!(result.is_ok());

        let output = result.unwrap();

        // Check timing analysis content
        assert!(output.contains("Timing Analysis Report"));
        assert!(output.contains("Average Response Time: 450ms"));
        assert!(output.contains("95th Percentile: 1800ms"));
        assert!(output.contains("99th Percentile: 2300ms"));
    }

    #[test]
    fn test_format_log_entries() {
        let formatter = TextFormatter::new();
        let entries = create_test_log_entries();

        let result = formatter.format_log_entries(&entries);
        assert!(result.is_ok());

        let output = result.unwrap();

        // Check log entries formatting
        assert!(output.contains("Log Entries (3 total)"));
        assert!(output.contains("[1] 2024-08-15 10:30:00 Statement:"));
        assert!(output.contains("[2] 2024-08-15 10:30:01 Error:"));
        assert!(output.contains("[3] 2024-08-15 10:30:02 Duration:"));
        assert!(output.contains("SELECT * FROM users WHERE active = true"));
        assert!(output.contains("relation \"missing_table\" does not exist"));
        assert!(output.contains("duration: 45.123 ms"));
    }

    #[test]
    fn test_format_log_entries_empty() {
        let formatter = TextFormatter::new();
        let entries = vec![];

        let result = formatter.format_log_entries(&entries);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.contains("Log Entries (0 total)"));
    }

    #[test]
    fn test_bold_function_no_color() {
        let result = pg_loggrep::output::text::bold("test", Some("red"), false);
        assert_eq!(result, "test");
    }

    #[test]
    fn test_bold_function_with_color() {
        let result = pg_loggrep::output::text::bold("test", Some("red"), true);
        assert!(result.contains("\x1b[31;1m"));
        assert!(result.contains("test"));
        assert!(result.contains("\x1b[0m"));
    }

    #[test]
    fn test_bold_function_different_colors() {
        let colors = vec![
            ("red", "\x1b[31;1m"),
            ("green", "\x1b[32;1m"),
            ("yellow", "\x1b[33;1m"),
            ("blue", "\x1b[34;1m"),
            ("magenta", "\x1b[35;1m"),
            ("cyan", "\x1b[36;1m"),
            ("white", "\x1b[37;1m"),
            ("unknown", "\x1b[37;1m"), // Default to white
        ];

        for (color, expected_code) in colors {
            let result = pg_loggrep::output::text::bold("test", Some(color), true);
            assert!(result.contains(expected_code));
        }
    }
}

#[cfg(test)]
mod json_formatter_tests {
    use super::*;

    #[test]
    fn test_json_formatter_new() {
        let formatter = JsonFormatter::new();
        assert!(!formatter.pretty); // Default should be compact
        assert_eq!(formatter.tool_version, env!("CARGO_PKG_VERSION"));
        assert!(formatter.log_files_processed.is_empty());
        assert_eq!(formatter.total_log_entries, 0);
    }

    #[test]
    fn test_json_formatter_with_pretty() {
        let formatter = JsonFormatter::new().with_pretty(true);
        assert!(formatter.pretty);

        let formatter = JsonFormatter::new().with_pretty(false);
        assert!(!formatter.pretty);
    }

    #[test]
    fn test_json_formatter_with_metadata() {
        let files = vec!["file1.log".to_string(), "file2.log".to_string()];
        let formatter = JsonFormatter::new()
            .with_metadata("1.0.0", files.clone(), 1000);

        assert_eq!(formatter.tool_version, "1.0.0");
        assert_eq!(formatter.log_files_processed, files);
        assert_eq!(formatter.total_log_entries, 1000);
    }

    #[test]
    fn test_format_basic_analysis() {
        let formatter = JsonFormatter::new();
        let analysis = create_test_analysis_result();

        let result = formatter.format(&analysis);
        assert!(result.is_ok());

        let json_str = result.unwrap();
        let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        // Check metadata
        assert!(json["metadata"]["analysis_timestamp"].is_string());
        assert_eq!(json["metadata"]["tool_version"], env!("CARGO_PKG_VERSION"));
        assert!(json["metadata"]["log_files_processed"].is_array());
        assert_eq!(json["metadata"]["total_log_entries"], 0);

        // Check summary
        assert_eq!(json["summary"]["total_queries"], 11);
        assert_eq!(json["summary"]["total_duration_ms"], 5500.0);
        assert_eq!(json["summary"]["avg_duration_ms"], 500.0);
        assert_eq!(json["summary"]["error_count"], 2);
        assert_eq!(json["summary"]["connection_count"], 3);

        // Check query analysis
        assert_eq!(json["query_analysis"]["by_type"]["SELECT"], 5);
        assert_eq!(json["query_analysis"]["by_type"]["INSERT"], 3);
        assert_eq!(json["query_analysis"]["by_type"]["UPDATE"], 2);
        assert_eq!(json["query_analysis"]["by_type"]["DELETE"], 1);
    }

    #[test]
    fn test_format_slowest_queries() {
        let formatter = JsonFormatter::new();
        let analysis = create_test_analysis_result();

        let result = formatter.format(&analysis);
        assert!(result.is_ok());

        let json_str = result.unwrap();
        let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        let slowest = &json["query_analysis"]["slowest_queries"];
        assert!(slowest.is_array());
        assert_eq!(slowest.as_array().unwrap().len(), 3);

        // Check first slowest query
        let first = &slowest[0];
        assert!(first["query"].as_str().unwrap().contains("SELECT * FROM large_table"));
        assert_eq!(first["duration_ms"], 2500.0);
        assert_eq!(first["count"], 1); // Default count
    }

    #[test]
    fn test_format_most_frequent_queries() {
        let formatter = JsonFormatter::new();
        let analysis = create_test_analysis_result();

        let result = formatter.format(&analysis);
        assert!(result.is_ok());

        let json_str = result.unwrap();
        let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        let frequent = &json["query_analysis"]["most_frequent"];
        assert!(frequent.is_array());
        assert_eq!(frequent.as_array().unwrap().len(), 4);

        // Check first most frequent query
        let first = &frequent[0];
        assert!(first["query"].as_str().unwrap().contains("SELECT * FROM users WHERE active"));
        assert_eq!(first["count"], 15);
        assert_eq!(first["avg_duration_ms"], 500.0); // Overall average
    }

    #[test]
    fn test_format_with_timing() {
        let formatter = JsonFormatter::new();
        let analysis = create_test_analysis_result();
        let timing = create_test_timing_analysis();

        let result = formatter.format_with_timing(&analysis, &timing);
        assert!(result.is_ok());

        let json_str = result.unwrap();
        let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        // Should have temporal analysis section
        assert!(json["temporal_analysis"].is_object());
        assert_eq!(json["temporal_analysis"]["average_response_time_ms"], 450);
        assert_eq!(json["temporal_analysis"]["p95_response_time_ms"], 1800);
        assert_eq!(json["temporal_analysis"]["p99_response_time_ms"], 2300);

        // Check hourly stats
        let hourly = &json["temporal_analysis"]["hourly_stats"];
        assert!(hourly.is_array());
        assert_eq!(hourly.as_array().unwrap().len(), 5);
    }

    #[test]
    fn test_format_pretty_printing() {
        let formatter = JsonFormatter::new().with_pretty(true);
        let analysis = create_test_analysis_result();

        let result = formatter.format(&analysis);
        assert!(result.is_ok());

        let json_str = result.unwrap();

        // Pretty printed JSON should contain newlines and indentation
        assert!(json_str.contains('\n'));
        assert!(json_str.contains("  ")); // Indentation
    }

    #[test]
    fn test_format_compact() {
        let formatter = JsonFormatter::new().with_pretty(false);
        let analysis = create_test_analysis_result();

        let result = formatter.format(&analysis);
        assert!(result.is_ok());

        let json_str = result.unwrap();

        // Compact JSON should not contain unnecessary whitespace
        assert!(!json_str.contains('\n'));
        assert!(!json_str.contains("  "));
    }

    #[test]
    fn test_format_empty_analysis() {
        let formatter = JsonFormatter::new();
        let analysis = AnalysisResult::new();

        let result = formatter.format(&analysis);
        assert!(result.is_ok());

        let json_str = result.unwrap();
        let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        // Check empty analysis values
        assert_eq!(json["summary"]["total_queries"], 0);
        assert_eq!(json["summary"]["total_duration_ms"], 0.0);
        assert_eq!(json["summary"]["avg_duration_ms"], 0.0);
        assert_eq!(json["summary"]["error_count"], 0);
        assert_eq!(json["summary"]["connection_count"], 0);

        // Empty arrays for queries
        assert!(json["query_analysis"]["slowest_queries"].as_array().unwrap().is_empty());
        assert!(json["query_analysis"]["most_frequent"].as_array().unwrap().is_empty());
        assert!(json["query_analysis"]["by_type"].as_object().unwrap().is_empty());
    }

    #[test]
    fn test_metadata_object() {
        let files = vec!["test1.log".to_string(), "test2.log".to_string()];
        let formatter = JsonFormatter::new()
            .with_metadata("2.0.0", files.clone(), 500);

        let metadata = formatter.metadata_object();

        assert!(metadata["analysis_timestamp"].is_string());
        assert_eq!(metadata["tool_version"], "2.0.0");
        assert_eq!(metadata["log_files_processed"], serde_json::json!(files));
        assert_eq!(metadata["total_log_entries"], 500);
    }

    #[test]
    fn test_json_serialization_roundtrip() {
        let formatter = JsonFormatter::new();
        let analysis = create_test_analysis_result();

        let result = formatter.format(&analysis);
        assert!(result.is_ok());

        let json_str = result.unwrap();

        // Should be valid JSON that can be parsed
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        // Should be able to serialize back to string
        let serialized = serde_json::to_string(&parsed).unwrap();
        assert!(!serialized.is_empty());
    }

    #[test]
    fn test_json_structure_completeness() {
        let formatter = JsonFormatter::new();
        let analysis = create_test_analysis_result();

        let result = formatter.format(&analysis);
        assert!(result.is_ok());

        let json_str = result.unwrap();
        let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        // Verify all expected top-level keys exist
        assert!(json["metadata"].is_object());
        assert!(json["summary"].is_object());
        assert!(json["query_analysis"].is_object());

        // Verify query_analysis structure
        let qa = &json["query_analysis"];
        assert!(qa["by_type"].is_object());
        assert!(qa["slowest_queries"].is_array());
        assert!(qa["most_frequent"].is_array());
    }
}

#[cfg(test)]
mod output_edge_cases_tests {
    use super::*;

    #[test]
    fn test_text_formatter_with_special_characters() {
        let formatter = TextFormatter::new();

        let mut analysis = AnalysisResult::new();
        analysis.total_queries = 1;
        analysis.slowest_queries = vec![
            ("SELECT * FROM \"table-with-dashes\" WHERE name = 'O''Reilly'".to_string(), 100.0),
        ];
        analysis.most_frequent_queries = vec![
            ("INSERT INTO logs (message) VALUES ('Error: \"Connection failed\"')".to_string(), 5),
        ];

        let result = formatter.format_query_analysis(&analysis);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.contains("table-with-dashes"));
        assert!(output.contains("O''Reilly"));
        assert!(output.contains("Connection failed"));
    }

    #[test]
    fn test_json_formatter_with_special_characters() {
        let formatter = JsonFormatter::new();

        let mut analysis = AnalysisResult::new();
        analysis.total_queries = 1;
        analysis.slowest_queries = vec![
            ("SELECT * FROM \"table-with-dashes\" WHERE name = 'O''Reilly'".to_string(), 100.0),
        ];

        let result = formatter.format(&analysis);
        assert!(result.is_ok());

        let json_str = result.unwrap();
        let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        // JSON should properly escape special characters
        let query = json["query_analysis"]["slowest_queries"][0]["query"].as_str().unwrap();
        assert!(query.contains("table-with-dashes"));
        assert!(query.contains("O''Reilly"));
    }

    #[test]
    fn test_text_formatter_with_very_long_queries() {
        let formatter = TextFormatter::new();

        let long_query = format!("SELECT {} FROM users", "column_name, ".repeat(100));
        let mut analysis = AnalysisResult::new();
        analysis.total_queries = 1;
        analysis.slowest_queries = vec![(long_query.clone(), 100.0)];

        let result = formatter.format_query_analysis(&analysis);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.contains(&long_query));
    }

    #[test]
    fn test_json_formatter_with_very_long_queries() {
        let formatter = JsonFormatter::new();

        let long_query = format!("SELECT {} FROM users", "column_name, ".repeat(100));
        let mut analysis = AnalysisResult::new();
        analysis.total_queries = 1;
        analysis.slowest_queries = vec![(long_query.clone(), 100.0)];

        let result = formatter.format(&analysis);
        assert!(result.is_ok());

        let json_str = result.unwrap();
        let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        let query = json["query_analysis"]["slowest_queries"][0]["query"].as_str().unwrap();
        assert_eq!(query, long_query);
    }

    #[test]
    fn test_text_formatter_with_unicode() {
        let formatter = TextFormatter::new();

        let mut analysis = AnalysisResult::new();
        analysis.total_queries = 1;
        analysis.slowest_queries = vec![
            ("SELECT * FROM users WHERE name = '测试用户'".to_string(), 100.0),
        ];

        let result = formatter.format_query_analysis(&analysis);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.contains("测试用户"));
    }

    #[test]
    fn test_json_formatter_with_unicode() {
        let formatter = JsonFormatter::new();

        let mut analysis = AnalysisResult::new();
        analysis.total_queries = 1;
        analysis.slowest_queries = vec![
            ("SELECT * FROM users WHERE name = '测试用户'".to_string(), 100.0),
        ];

        let result = formatter.format(&analysis);
        assert!(result.is_ok());

        let json_str = result.unwrap();
        let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        let query = json["query_analysis"]["slowest_queries"][0]["query"].as_str().unwrap();
        assert!(query.contains("测试用户"));
    }

    #[test]
    fn test_text_formatter_with_zero_values() {
        let formatter = TextFormatter::new();

        let mut analysis = AnalysisResult::new();
        analysis.total_queries = 0;
        analysis.total_duration = 0.0;
        analysis.average_duration = 0.0;

        let result = formatter.format_query_analysis(&analysis);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.contains("Total Queries: 0"));
        assert!(output.contains("Total Duration: 0.00 ms"));
        assert!(output.contains("Average Duration: 0.00 ms"));
    }

    #[test]
    fn test_json_formatter_with_zero_values() {
        let formatter = JsonFormatter::new();

        let analysis = AnalysisResult::new(); // All zeros by default

        let result = formatter.format(&analysis);
        assert!(result.is_ok());

        let json_str = result.unwrap();
        let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(json["summary"]["total_queries"], 0);
        assert_eq!(json["summary"]["total_duration_ms"], 0.0);
        assert_eq!(json["summary"]["avg_duration_ms"], 0.0);
    }

    #[test]
    fn test_text_formatter_with_large_numbers() {
        let formatter = TextFormatter::new();

        let mut analysis = AnalysisResult::new();
        analysis.total_queries = 1_000_000;
        analysis.total_duration = 999_999.99;
        analysis.average_duration = 999.999;

        let result = formatter.format_query_analysis(&analysis);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert!(output.contains("Total Queries: 1000000"));
        assert!(output.contains("Total Duration: 999999.99 ms"));
        assert!(output.contains("Average Duration: 999.999 ms"));
    }

    #[test]
    fn test_json_formatter_with_large_numbers() {
        let formatter = JsonFormatter::new();

        let mut analysis = AnalysisResult::new();
        analysis.total_queries = 1_000_000;
        analysis.total_duration = 999_999.99;
        analysis.average_duration = 999.999;

        let result = formatter.format(&analysis);
        assert!(result.is_ok());

        let json_str = result.unwrap();
        let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(json["summary"]["total_queries"], 1_000_000);
        assert_eq!(json["summary"]["total_duration_ms"], 999_999.99);
        assert_eq!(json["summary"]["avg_duration_ms"], 999.999);
    }
}

#[cfg(test)]
mod output_performance_tests {
    use super::*;

    #[test]
    fn test_text_formatter_performance_large_dataset() {
        let formatter = TextFormatter::new();

        // Create analysis with many queries
        let mut analysis = AnalysisResult::new();
        analysis.total_queries = 10000;

        // Add many slowest queries
        for i in 0..1000 {
            analysis.slowest_queries.push((
                format!("SELECT * FROM table_{} WHERE id = ?", i),
                (1000 - i) as f64,
            ));
        }

        // Add many frequent queries
        for i in 0..1000 {
            analysis.most_frequent_queries.push((
                format!("INSERT INTO table_{} VALUES (?)", i),
                (1000 - i) as u64,
            ));
        }

        let start = std::time::Instant::now();
        let result = formatter.format_query_analysis(&analysis);
        let duration = start.elapsed();

        assert!(result.is_ok());
        assert!(duration.as_millis() < 1000, "Text formatting took too long: {:?}", duration);
    }

    #[test]
    fn test_json_formatter_performance_large_dataset() {
        let formatter = JsonFormatter::new();

        // Create analysis with many queries
        let mut analysis = AnalysisResult::new();
        analysis.total_queries = 10000;

        // Add many slowest queries
        for i in 0..1000 {
            analysis.slowest_queries.push((
                format!("SELECT * FROM table_{} WHERE id = ?", i),
                (1000 - i) as f64,
            ));
        }

        // Add many frequent queries
        for i in 0..1000 {
            analysis.most_frequent_queries.push((
                format!("INSERT INTO table_{} VALUES (?)", i),
                (1000 - i) as u64,
            ));
        }

        let start = std::time::Instant::now();
        let result = formatter.format(&analysis);
        let duration = start.elapsed();

        assert!(result.is_ok());
        assert!(duration.as_millis() < 1000, "JSON formatting took too long: {:?}", duration);

        // Verify the JSON is still valid
        let json_str = result.unwrap();
        let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(json["summary"]["total_queries"], 10000);
    }

    #[test]
    fn test_text_formatter_memory_usage() {
        let formatter = TextFormatter::new();

        // Create analysis with very long query strings
        let mut analysis = AnalysisResult::new();
        analysis.total_queries = 100;

        let very_long_query = format!("SELECT {} FROM users", "very_long_column_name, ".repeat(1000));
        for i in 0..100 {
            analysis.slowest_queries.push((
                format!("{} WHERE id = {}", very_long_query, i),
                (100 - i) as f64,
            ));
        }

        let result = formatter.format_query_analysis(&analysis);
        assert!(result.is_ok());

        // Should handle large strings without memory issues
        let output = result.unwrap();
        assert!(output.len() > 100000); // Should be a large output
        assert!(output.contains("very_long_column_name"));
    }

    #[test]
    fn test_json_formatter_memory_usage() {
        let formatter = JsonFormatter::new();

        // Create analysis with very long query strings
        let mut analysis = AnalysisResult::new();
        analysis.total_queries = 100;

        let very_long_query = format!("SELECT {} FROM users", "very_long_column_name, ".repeat(1000));
        for i in 0..100 {
            analysis.slowest_queries.push((
                format!("{} WHERE id = {}", very_long_query, i),
                (100 - i) as f64,
            ));
        }

        let result = formatter.format(&analysis);
        assert!(result.is_ok());

        // Should handle large strings without memory issues
        let json_str = result.unwrap();
        let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(json["summary"]["total_queries"], 100);

        // Verify one of the long queries is properly serialized
        let first_query = json["query_analysis"]["slowest_queries"][0]["query"].as_str().unwrap();
        assert!(first_query.contains("very_long_column_name"));
    }
}
