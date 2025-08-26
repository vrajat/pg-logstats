//! Unit tests for PostgreSQL stderr log parser
//!
//! Tests various log line formats, edge cases, and parser functionality in isolation

use chrono::DateTime;
use pg_logstats::parsers::stderr::StderrParser;
use pg_logstats::LogLevel;

/// Helper function to create test log lines with various formats
fn create_test_lines() -> Vec<String> {
    vec![
        // Standard statement log
        "2024-08-15 10:30:15.123 UTC [12345] postgres@testdb psql: LOG:  statement: SELECT * FROM users WHERE active = true;".to_string(),

        // Duration log
        "2024-08-15 10:30:15.456 UTC [12345] postgres@testdb psql: LOG:  duration: 45.123 ms".to_string(),

        // Error log
        "2024-08-15 10:30:16.789 UTC [12346] admin@analytics pgbench: ERROR:  relation \"missing_table\" does not exist".to_string(),

        // Warning log
        "2024-08-15 10:30:17.012 UTC [12347] postgres@testdb psql: WARNING:  there is no transaction in progress".to_string(),

        // Parameterized query
        "2024-08-15 10:30:18.345 UTC [12348] postgres@testdb psql: LOG:  statement: UPDATE products SET price = $1 WHERE id = $2".to_string(),

        // Multi-line statement start
        "2024-08-15 10:30:19.678 UTC [12349] postgres@testdb psql: LOG:  statement: SELECT u.name, p.title".to_string(),

        // Continuation lines (no timestamp)
        "    FROM users u".to_string(),
        "    JOIN posts p ON u.id = p.user_id".to_string(),
        "    WHERE u.active = true".to_string(),
        "    ORDER BY p.created_at DESC;".to_string(),

        // Duration for multi-line statement
        "2024-08-15 10:30:19.890 UTC [12349] postgres@testdb psql: LOG:  duration: 12.345 ms".to_string(),

        // Empty line
        "".to_string(),

        // Invalid/unparseable line
        "This is not a PostgreSQL log line".to_string(),

        // Different timestamp formats
        "2024-08-15 10:30:20 UTC [12350] postgres@testdb psql: LOG:  statement: SELECT NOW();".to_string(),

        // Different log levels
        "2024-08-15 10:30:21.111 UTC [12351] postgres@testdb psql: INFO:  checkpoint starting: time".to_string(),

        // FATAL error
        "2024-08-15 10:30:22.222 UTC [12352] postgres@testdb psql: FATAL:  database \"nonexistent\" does not exist".to_string(),

        // PANIC error
        "2024-08-15 10:30:23.333 UTC [12353] postgres@testdb psql: PANIC:  could not write to file".to_string(),

        // Complex query with special characters
        "2024-08-15 10:30:24.444 UTC [12354] postgres@testdb psql: LOG:  statement: SELECT * FROM \"user-table\" WHERE name LIKE '%John''s%' AND age > 25;".to_string(),

        // Query with bind parameters and execution
        "2024-08-15 10:30:25.555 UTC [12355] postgres@testdb psql: LOG:  execute <unnamed>: SELECT * FROM users WHERE id = $1".to_string(),

        // Very long query (truncated)
        format!("2024-08-15 10:30:26.666 UTC [12356] postgres@testdb psql: LOG:  statement: SELECT {} FROM users;", "column_name, ".repeat(100)),
    ]
}

#[cfg(test)]
mod parser_unit_tests {
    use super::*;

    #[test]
    fn test_parse_simple_statement() {
        let mut parser = StderrParser::new();
        let line = "2024-08-15 10:30:15.123 UTC [12345] postgres@testdb psql: LOG:  statement: SELECT * FROM users WHERE active = true;";

        let result = parser.parse_line(line).unwrap();
        assert!(result.is_some());

        let entry = result.unwrap();
        assert_eq!(entry.process_id, "12345");
        assert_eq!(entry.user, Some("postgres".to_string()));
        assert_eq!(entry.database, Some("testdb".to_string()));
        assert_eq!(entry.application_name, Some("psql".to_string()));
        assert_eq!(entry.message_type, LogLevel::Statement);
        assert!(entry.queries.is_some());
        let queries = entry.queries.unwrap();
        assert_eq!(queries.len(), 1);
        assert_eq!(
            queries[0].normalized_query,
            "SELECT * FROM users WHERE active = ?"
        );
        assert!(entry.duration.is_none());
    }

    #[test]
    fn test_parse_duration_log() {
        let mut parser = StderrParser::new();
        let line =
            "2024-08-15 10:30:15.456 UTC [12345] postgres@testdb psql: LOG:  duration: 45.123 ms";

        let result = parser.parse_line(line).unwrap();
        assert!(result.is_some());

        let entry = result.unwrap();
        assert_eq!(entry.message_type, LogLevel::Duration);
        assert_eq!(entry.duration, Some(45.123));
        assert!(entry.queries.is_none());
    }

    #[test]
    fn test_parse_error_log() {
        let mut parser = StderrParser::new();
        let line = "2024-08-15 10:30:16.789 UTC [12346] admin@analytics pgbench: ERROR:  relation \"missing_table\" does not exist";

        let result = parser.parse_line(line).unwrap();
        assert!(result.is_some());

        let entry = result.unwrap();
        assert_eq!(entry.message_type, LogLevel::Error);
        assert_eq!(entry.user, Some("admin".to_string()));
        assert_eq!(entry.database, Some("analytics".to_string()));
        assert_eq!(entry.application_name, Some("pgbench".to_string()));
        assert!(entry
            .message
            .contains("relation \"missing_table\" does not exist"));
        assert!(entry.queries.is_none());
        assert!(entry.duration.is_none());
    }

    #[test]
    fn test_parse_warning_log() {
        let mut parser = StderrParser::new();
        let line = "2024-08-15 10:30:17.012 UTC [12347] postgres@testdb psql: WARNING:  there is no transaction in progress";

        let result = parser.parse_line(line).unwrap();
        assert!(result.is_some());

        let entry = result.unwrap();
        assert_eq!(entry.message_type, LogLevel::Warning);
        assert!(entry
            .message
            .contains("there is no transaction in progress"));
    }

    #[test]
    fn test_parse_parameterized_query() {
        let mut parser = StderrParser::new();
        let line = "2024-08-15 10:30:18.345 UTC [12348] postgres@testdb psql: LOG:  statement: UPDATE products SET price = $1 WHERE id = $2";

        let result = parser.parse_line(line).unwrap();
        assert!(result.is_some());

        let entry = result.unwrap();
        assert_eq!(entry.message_type, LogLevel::Statement);
        // Query should be normalized with parameters replaced
        assert!(entry.queries.is_some());
        let queries = entry.queries.unwrap();
        assert_eq!(queries.len(), 1);
        assert_eq!(
            queries[0].normalized_query,
            "UPDATE products SET price = ? WHERE id = ?".to_string()
        );
    }

    #[test]
    fn test_parse_multi_line_statement() {
        let lines = vec![
            "2024-08-15 10:30:19.678 UTC [12349] postgres@testdb psql: LOG:  statement: SELECT u.name, p.title",
            "    FROM users u",
            "    JOIN posts p ON u.id = p.user_id",
            "    WHERE u.active = true",
            "    ORDER BY p.created_at DESC;",
            "2024-08-15 10:30:19.890 UTC [12349] postgres@testdb psql: LOG:  duration: 12.345 ms",
        ];

        let parser = StderrParser::new();
        let result = parser.parse_lines(&lines.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert!(result.is_ok());

        let entries = result.unwrap();
        assert_eq!(entries.len(), 2); // Statement and duration entries

        let statement_entry = &entries[0];
        let duration_entry = &entries[1];

        assert_eq!(statement_entry.message_type, LogLevel::Statement);
        assert_eq!(duration_entry.message_type, LogLevel::Duration);
        assert_eq!(duration_entry.duration, Some(12.345));

        // Multi-line query should be properly assembled
        assert!(statement_entry.queries.is_some());
        let queries = statement_entry.queries.as_ref().unwrap();
        assert_eq!(queries.len(), 1);
        assert!(queries[0]
            .normalized_query
            .contains("SELECT u.name, p.title"));
    }

    #[test]
    fn test_parse_empty_line() {
        let mut parser = StderrParser::new();
        let result = parser.parse_line("").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_unparseable_line() {
        let mut parser = StderrParser::new();
        let result = parser
            .parse_line("This is not a PostgreSQL log line")
            .unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_continuation_line_without_pending() {
        let mut parser = StderrParser::new();
        let result = parser.parse_line("    FROM users u").unwrap();
        assert!(result.is_none()); // Should skip continuation lines without pending statement
    }

    #[test]
    fn test_parse_different_timestamp_formats() {
        let mut parser = StderrParser::new();

        // With milliseconds
        let line1 =
            "2024-08-15 10:30:15.123 UTC [12345] postgres@testdb psql: LOG:  statement: SELECT 1;";
        let result1 = parser.parse_line(line1).unwrap();
        assert!(result1.is_some());

        // Without milliseconds
        let line2 =
            "2024-08-15 10:30:20 UTC [12350] postgres@testdb psql: LOG:  statement: SELECT 2;";
        let result2 = parser.parse_line(line2).unwrap();
        assert!(result2.is_some());
    }

    #[test]
    fn test_parse_different_log_levels() {
        let mut parser = StderrParser::new();

        let test_cases = vec![
            ("INFO", LogLevel::Info),
            ("WARNING", LogLevel::Warning),
            ("ERROR", LogLevel::Error),
            ("FATAL", LogLevel::Fatal),
            ("PANIC", LogLevel::Panic),
            ("LOG", LogLevel::Log),
        ];

        for (level_str, expected_level) in test_cases {
            let line = format!(
                "2024-08-15 10:30:15.123 UTC [12345] postgres@testdb psql: {}:  test message",
                level_str
            );
            let result = parser.parse_line(&line).unwrap();
            assert!(result.is_some());

            let entry = result.unwrap();
            assert_eq!(entry.message_type, expected_level);
        }
    }

    #[test]
    fn test_parse_complex_query_with_special_characters() {
        let mut parser = StderrParser::new();
        let line = "2024-08-15 10:30:24.444 UTC [12354] postgres@testdb psql: LOG:  statement: SELECT * FROM \"user-table\" WHERE name LIKE '%John''s%' AND age > 25;";

        let result = parser.parse_line(line).unwrap();
        assert!(result.is_some());

        let entry = result.unwrap();
        assert_eq!(entry.message_type, LogLevel::Statement);
        assert!(entry.queries.is_some());
        let queries = entry.queries.unwrap();
        assert_eq!(queries.len(), 1);
        assert!(queries[0].normalized_query.contains("user-table"));
    }

    #[test]
    fn test_parse_execute_statement() {
        let mut parser = StderrParser::new();
        let line = "2024-08-15 10:30:25.555 UTC [12355] postgres@testdb psql: LOG:  execute <unnamed>: SELECT * FROM users WHERE id = $1";

        let result = parser.parse_line(line).unwrap();
        assert!(result.is_some());

        let entry = result.unwrap();
        assert_eq!(entry.message_type, LogLevel::Log);
        assert!(entry.message.contains("execute <unnamed>"));
    }

    #[test]
    fn test_parse_lines_with_mixed_content() {
        let lines = vec![
            "2024-08-15 10:30:15.123 UTC [12345] postgres@testdb psql: LOG:  statement: SELECT * FROM users;",
            "This is not a valid log line",
            "",
            "2024-08-15 10:30:15.456 UTC [12345] postgres@testdb psql: LOG:  duration: 45.123 ms",
            "    continuation line without pending statement",
        ];

        let parser = StderrParser::new();
        let result = parser.parse_lines(&lines.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert!(result.is_ok());

        let entries = result.unwrap();
        assert_eq!(entries.len(), 2); // Should parse 2 valid lines, skip invalid ones
    }

    #[test]
    fn test_extract_duration() {
        let parser = StderrParser::new();

        assert_eq!(parser.extract_duration("duration: 45.123 ms"), Some(45.123));
        assert_eq!(parser.extract_duration("duration: 1000 ms"), Some(1000.0));
        assert_eq!(parser.extract_duration("duration: 0.001 ms"), Some(0.001));
        assert_eq!(parser.extract_duration("no duration here"), None);
        assert_eq!(parser.extract_duration("duration: invalid ms"), None);
    }

    #[test]
    fn test_timestamp_parsing_edge_cases() {
        let parser = StderrParser::new();

        // Test various timestamp formats
        let test_cases = vec![
            ("2024-08-15 10:30:15.123", "UTC"),
            ("2024-08-15 10:30:15", "UTC"),
            ("2024-12-31 23:59:59.999", "UTC"),
            ("2024-01-01 00:00:00", "UTC"),
        ];

        for (timestamp_str, timezone) in test_cases {
            let result = parser.parse_timestamp(timestamp_str, timezone);
            assert!(
                result.is_ok(),
                "Failed to parse timestamp: {}",
                timestamp_str
            );
        }
    }

    #[test]
    fn test_timestamp_parsing_invalid() {
        let parser = StderrParser::new();

        let invalid_timestamps = vec![
            "invalid-timestamp",
            "2024-13-01 10:30:15", // Invalid month
            "2024-08-32 10:30:15", // Invalid day
            "2024-08-15 25:30:15", // Invalid hour
        ];

        for timestamp_str in invalid_timestamps {
            let result = parser.parse_timestamp(timestamp_str, "UTC");
            assert!(
                result.is_err(),
                "Should fail to parse invalid timestamp: {}",
                timestamp_str
            );
        }
    }

    #[test]
    fn test_regex_patterns() {
        let parser = StderrParser::new();

        // Test log line regex
        let valid_line = "2024-08-15 10:30:15.123 UTC [12345] postgres@testdb psql: LOG:  statement: SELECT * FROM users;";
        assert!(parser.log_line_regex.is_match(valid_line));

        let invalid_line = "This is not a log line";
        assert!(!parser.log_line_regex.is_match(invalid_line));

        // Test duration regex
        assert!(parser.duration_regex().is_match("duration: 45.123 ms"));
        assert!(parser.duration_regex().is_match("duration: 1000 ms"));
        assert!(!parser.duration_regex().is_match("no duration here"));

        // Test parameter regex
        assert!(parser
            .parameter_regex()
            .is_match("SELECT * FROM users WHERE id = $1"));
        assert!(parser
            .parameter_regex()
            .is_match("UPDATE users SET name = $1, email = $2"));
        assert!(!parser.parameter_regex().is_match("SELECT * FROM users"));
    }

    #[test]
    fn test_parser_state_management() {
        let mut parser = StderrParser::new();

        // Start a multi-line statement
        let line1 = "2024-08-15 10:30:19.678 UTC [12349] postgres@testdb psql: LOG:  statement: SELECT u.name";
        let result1 = parser.parse_line(line1).unwrap();
        assert!(result1.is_some());

        // Add continuation line
        let line2 = "    FROM users u";
        let result2 = parser.parse_line(line2).unwrap();
        assert!(result2.is_none()); // Continuation lines return None

        // Add another continuation line
        let line3 = "    WHERE u.active = true;";
        let result3 = parser.parse_line(line3).unwrap();
        assert!(result3.is_none());

        // Start a new statement (should finalize the previous one)
        let line4 = "2024-08-15 10:30:20.000 UTC [12350] postgres@testdb psql: LOG:  statement: SELECT COUNT(*) FROM posts;";
        let result4 = parser.parse_line(line4).unwrap();
        assert!(result4.is_some());
    }

    #[test]
    fn test_performance_with_large_input() {
        let parser = StderrParser::new();

        // Create a large number of log lines
        let mut lines = Vec::new();
        for i in 0..1000 {
            lines.push(format!(
                "2024-08-15 10:30:{:02}.{:03} UTC [{}] postgres@testdb psql: LOG:  statement: SELECT * FROM users WHERE id = {};",
                i % 60, i % 1000, 12345 + i, i
            ));
        }

        let start = std::time::Instant::now();
        let result = parser.parse_lines(&lines);
        let duration = start.elapsed();

        assert!(result.is_ok());
        let entries = result.unwrap();
        assert_eq!(entries.len(), 1000);

        // Should complete within reasonable time (adjust threshold as needed)
        assert!(
            duration.as_millis() < 1000,
            "Parsing took too long: {:?}",
            duration
        );
    }

    #[test]
    fn test_memory_usage_with_large_queries() {
        let mut parser = StderrParser::new();

        // Create a very long query
        let long_query = format!("SELECT {} FROM users;", "column_name, ".repeat(10000));
        let line = format!(
            "2024-08-15 10:30:15.123 UTC [12345] postgres@testdb psql: LOG:  statement: {}",
            long_query
        );

        let result = parser.parse_line(&line).unwrap();
        assert!(result.is_some());

        let entry = result.unwrap();
        assert!(entry.queries.is_some());
        let queries = entry.queries.unwrap();
        assert_eq!(queries.len(), 1);
        // Query should be normalized and not cause memory issues
        assert!(queries[0].normalized_query.len() > 0);
    }
}

#[cfg(test)]
mod property_based_tests {
    use super::*;
    use std::collections::HashSet;

    /// Property: All valid log entries should have required fields
    #[test]
    fn property_valid_entries_have_required_fields() {
        let parser = StderrParser::new();
        let test_lines = create_test_lines();

        let result = parser.parse_lines(&test_lines);
        assert!(result.is_ok());

        let entries = result.unwrap();
        for entry in entries {
            // All entries should have timestamp and process_id
            assert!(!entry.process_id.is_empty());
            assert!(entry.timestamp > DateTime::from_timestamp(0, 0).unwrap());

            // Statement entries should have queries
            if entry.message_type == LogLevel::Statement {
                assert!(entry.queries.is_some());
            }

            // Duration entries should have duration values
            if entry.message_type == LogLevel::Duration {
                assert!(entry.duration.is_some());
                assert!(entry.duration.unwrap() >= 0.0);
            }
        }
    }

    /// Property: Parser should handle any sequence of valid log lines
    #[test]
    fn property_parser_handles_any_valid_sequence() {
        let base_lines = vec![
            "2024-08-15 10:30:15.123 UTC [12345] postgres@testdb psql: LOG:  statement: SELECT * FROM users;",
            "2024-08-15 10:30:15.456 UTC [12345] postgres@testdb psql: LOG:  duration: 45.123 ms",
            "2024-08-15 10:30:16.789 UTC [12346] admin@analytics pgbench: ERROR:  relation \"missing_table\" does not exist",
        ];

        // Test different permutations
        let permutations = vec![vec![0, 1, 2], vec![2, 0, 1], vec![1, 2, 0], vec![0, 2, 1]];

        for perm in permutations {
            let lines: Vec<String> = perm.iter().map(|&i| base_lines[i].to_string()).collect();
            let parser = StderrParser::new();
            let result = parser.parse_lines(&lines);

            assert!(result.is_ok());
            let entries = result.unwrap();
            assert_eq!(entries.len(), 3);
        }
    }

    /// Property: Unique process IDs should be preserved
    #[test]
    fn property_process_ids_preserved() {
        let parser = StderrParser::new();
        let test_lines = create_test_lines();

        let result = parser.parse_lines(&test_lines);
        assert!(result.is_ok());

        let entries = result.unwrap();
        let mut process_ids = HashSet::new();

        for entry in entries {
            process_ids.insert(entry.process_id.clone());
        }

        // Should have multiple unique process IDs from test data
        assert!(process_ids.len() > 1);

        // All process IDs should be numeric strings
        for pid in process_ids {
            assert!(
                pid.parse::<u32>().is_ok(),
                "Process ID should be numeric: {}",
                pid
            );
        }
    }
}
