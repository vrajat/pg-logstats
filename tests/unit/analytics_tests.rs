//! Unit tests for query analytics functionality
//!
//! Tests query analysis, classification, normalization, and performance metrics

use chrono::{DateTime, TimeZone, Utc};
use pg_logstats::analytics::queries::{QueryAnalyzer, QueryMetrics};
use pg_logstats::{LogEntry, LogLevel};
use pg_logstats::sql::{QueryType, Query};
use std::collections::HashMap;

/// Helper function to create test log entries
fn create_test_entry(
    timestamp: DateTime<Utc>,
    message_type: LogLevel,
    query: Option<String>,
    duration: Option<f64>,
    process_id: Option<&str>,
    user: Option<&str>,
    database: Option<&str>,
) -> LogEntry {
    LogEntry {
        timestamp,
        process_id: process_id.unwrap_or("12345").to_string(),
        user: user.map(|u| u.to_string()),
        database: database.map(|d| d.to_string()),
        client_host: None,
        application_name: Some("psql".to_string()),
        message_type,
        message: query
            .as_ref()
            .map_or("test message".to_string(), |q| format!("statement: {}", q)),
        queries: Query::from_sql(query.as_deref().unwrap_or("")).ok(),
        duration,
    }
}

/// Helper function to create a set of diverse test entries
fn create_diverse_test_entries() -> Vec<LogEntry> {
    let base_time = Utc.with_ymd_and_hms(2024, 8, 15, 10, 30, 0).unwrap();

    vec![
        // SELECT queries with different durations
        create_test_entry(
            base_time,
            LogLevel::Statement,
            Some("SELECT * FROM users WHERE active = true".to_string()),
            Some(50.0),
            Some("12345"),
            Some("postgres"),
            Some("testdb"),
        ),
        create_test_entry(
            base_time + chrono::Duration::seconds(1),
            LogLevel::Statement,
            Some("SELECT COUNT(*) FROM orders".to_string()),
            Some(25.0),
            Some("12346"),
            Some("admin"),
            Some("analytics"),
        ),
        create_test_entry(
            base_time + chrono::Duration::seconds(2),
            LogLevel::Statement,
            Some(
                "SELECT u.name, u.email FROM users u JOIN profiles p ON u.id = p.user_id"
                    .to_string(),
            ),
            Some(150.0),
            Some("12347"),
            Some("postgres"),
            Some("testdb"),
        ),
        // INSERT queries
        create_test_entry(
            base_time + chrono::Duration::seconds(3),
            LogLevel::Statement,
            Some("INSERT INTO users (name, email) VALUES ('John', 'john@example.com')".to_string()),
            Some(10.0),
            Some("12348"),
            Some("app_user"),
            Some("app_db"),
        ),
        create_test_entry(
            base_time + chrono::Duration::seconds(4),
            LogLevel::Statement,
            Some("INSERT INTO orders (user_id, total) VALUES (1, 99.99)".to_string()),
            Some(15.0),
            Some("12349"),
            Some("app_user"),
            Some("app_db"),
        ),
        // UPDATE queries
        create_test_entry(
            base_time + chrono::Duration::seconds(5),
            LogLevel::Statement,
            Some("UPDATE users SET last_login = NOW() WHERE id = 1".to_string()),
            Some(30.0),
            Some("12350"),
            Some("postgres"),
            Some("testdb"),
        ),
        // DELETE query
        create_test_entry(
            base_time + chrono::Duration::seconds(6),
            LogLevel::Statement,
            Some("DELETE FROM sessions WHERE expires_at < NOW()".to_string()),
            Some(75.0),
            Some("12351"),
            Some("cleanup_job"),
            Some("testdb"),
        ),
        // DDL queries
        create_test_entry(
            base_time + chrono::Duration::seconds(7),
            LogLevel::Statement,
            Some("CREATE INDEX idx_users_email ON users(email)".to_string()),
            Some(2000.0), // Slow DDL operation
            Some("12352"),
            Some("admin"),
            Some("testdb"),
        ),
        create_test_entry(
            base_time + chrono::Duration::seconds(8),
            LogLevel::Statement,
            Some("DROP TABLE temp_data".to_string()),
            Some(100.0),
            Some("12353"),
            Some("admin"),
            Some("testdb"),
        ),
        // Other queries
        create_test_entry(
            base_time + chrono::Duration::seconds(9),
            LogLevel::Statement,
            Some("BEGIN".to_string()),
            Some(1.0),
            Some("12354"),
            Some("postgres"),
            Some("testdb"),
        ),
        create_test_entry(
            base_time + chrono::Duration::seconds(10),
            LogLevel::Statement,
            Some("COMMIT".to_string()),
            Some(2.0),
            Some("12354"),
            Some("postgres"),
            Some("testdb"),
        ),
        // Error entries
        create_test_entry(
            base_time + chrono::Duration::seconds(11),
            LogLevel::Error,
            None,
            None,
            Some("12355"),
            Some("postgres"),
            Some("testdb"),
        ),
        create_test_entry(
            base_time + chrono::Duration::seconds(12),
            LogLevel::Error,
            None,
            None,
            Some("12356"),
            Some("app_user"),
            Some("app_db"),
        ),
        // Connection-related entries
        create_test_entry(
            base_time + chrono::Duration::seconds(13),
            LogLevel::Log,
            None,
            None,
            Some("12357"),
            Some("postgres"),
            Some("testdb"),
        ),
        // Duplicate queries for frequency testing
        create_test_entry(
            base_time + chrono::Duration::seconds(14),
            LogLevel::Statement,
            Some("SELECT * FROM users WHERE active = true".to_string()),
            Some(45.0),
            Some("12358"),
            Some("postgres"),
            Some("testdb"),
        ),
        create_test_entry(
            base_time + chrono::Duration::seconds(15),
            LogLevel::Statement,
            Some("SELECT * FROM users WHERE active = true".to_string()),
            Some(55.0),
            Some("12359"),
            Some("postgres"),
            Some("testdb"),
        ),
    ]
}

#[cfg(test)]
mod analytics_unit_tests {
    use super::*;

    #[test]
    fn test_query_analyzer_new() {
        let analyzer = QueryAnalyzer::new();

        // Test default settings
        assert_eq!(analyzer.slow_query_threshold(), 1000.0);
        assert_eq!(analyzer.max_slow_queries(), 10);
        assert_eq!(analyzer.max_frequent_queries(), 20);
    }

    #[test]
    fn test_query_analyzer_with_settings() {
        let analyzer = QueryAnalyzer::with_settings(500.0, 5, 15);

        assert_eq!(analyzer.slow_query_threshold(), 500.0);
        assert_eq!(analyzer.max_slow_queries(), 5);
        assert_eq!(analyzer.max_frequent_queries(), 15);
    }

    #[test]
    fn test_classify_query_select() {
        let analyzer = QueryAnalyzer::new();

        let select_queries = vec![
            "SELECT * FROM users",
            "select id, name from products",
            "  SELECT COUNT(*) FROM orders  ",
            "SELECT u.name FROM users u JOIN profiles p ON u.id = p.user_id",
        ];

        for query in select_queries {
            assert_eq!(analyzer.classify_query(query), QueryType::Select);
        }
    }

    #[test]
    fn test_classify_query_insert() {
        let analyzer = QueryAnalyzer::new();

        let insert_queries = vec![
            "INSERT INTO users (name) VALUES ('John')",
            "insert into products values (1, 'Product')",
            "  INSERT INTO orders SELECT * FROM temp_orders  ",
        ];

        for query in insert_queries {
            assert_eq!(analyzer.classify_query(query), QueryType::Insert);
        }
    }

    #[test]
    fn test_classify_query_update() {
        let analyzer = QueryAnalyzer::new();

        let update_queries = vec![
            "UPDATE users SET name = 'Jane'",
            "update products set price = 99.99",
            "  UPDATE orders SET status = 'completed' WHERE id = 1  ",
        ];

        for query in update_queries {
            assert_eq!(analyzer.classify_query(query), QueryType::Update);
        }
    }

    #[test]
    fn test_classify_query_delete() {
        let analyzer = QueryAnalyzer::new();

        let delete_queries = vec![
            "DELETE FROM users WHERE id = 1",
            "delete from temp_data",
            "  DELETE FROM sessions WHERE expires_at < NOW()  ",
        ];

        for query in delete_queries {
            assert_eq!(analyzer.classify_query(query), QueryType::Delete);
        }
    }

    #[test]
    fn test_classify_query_ddl() {
        let analyzer = QueryAnalyzer::new();

        let ddl_queries = vec![
            "CREATE TABLE users (id INT)",
            "DROP TABLE temp_data",
            "ALTER TABLE users ADD COLUMN email VARCHAR(255)",
            "TRUNCATE TABLE logs",
            "GRANT SELECT ON users TO readonly_user",
            "REVOKE INSERT ON products FROM app_user",
            "create index idx_users_email on users(email)",
        ];

        for query in ddl_queries {
            assert_eq!(analyzer.classify_query(query), QueryType::DDL);
        }
    }

    #[test]
    fn test_classify_query_other() {
        let analyzer = QueryAnalyzer::new();

        let other_queries = vec![
            "BEGIN",
            "COMMIT",
            "ROLLBACK",
            "EXPLAIN SELECT * FROM users",
            "ANALYZE TABLE users",
            "VACUUM users",
        ];

        for query in other_queries {
            assert_eq!(analyzer.classify_query(query), QueryType::Other);
        }
    }

    #[test]
    fn test_normalize_query_parameters() {
        let analyzer = QueryAnalyzer::new();

        let test_cases = vec![
            (
                "SELECT * FROM users WHERE id = $1",
                "SELECT * FROM users WHERE id = ?",
            ),
            (
                "UPDATE users SET name = $1, email = $2 WHERE id = $3",
                "UPDATE users SET name = ?, email = ? WHERE id = ?",
            ),
            (
                "INSERT INTO users VALUES ($1, $2, $3)",
                "INSERT INTO users VALUES (?, ?, ?)",
            ),
        ];

        for (input, expected) in test_cases {
            let result = analyzer.normalize_query(input);
            assert_eq!(result, expected);
        }
    }

    #[test]
    fn test_normalize_query_numeric_literals() {
        let analyzer = QueryAnalyzer::new();

        let test_cases = vec![
            (
                "SELECT * FROM users WHERE age > 25",
                "SELECT * FROM users WHERE age > N",
            ),
            (
                "UPDATE products SET price = 99.99",
                "UPDATE products SET price = N",
            ),
            (
                "SELECT * FROM orders WHERE total BETWEEN 10.5 AND 100",
                "SELECT * FROM orders WHERE total BETWEEN N AND N",
            ),
        ];

        for (input, expected) in test_cases {
            let result = analyzer.normalize_query(input);
            assert_eq!(result, expected);
        }
    }

    #[test]
    fn test_normalize_query_string_literals() {
        let analyzer = QueryAnalyzer::new();

        let test_cases = vec![
            (
                "SELECT * FROM users WHERE name = 'John'",
                "SELECT * FROM users WHERE name = S",
            ),
            (
                "INSERT INTO users VALUES ('John', 'john@example.com')",
                "INSERT INTO users VALUES (S, S)",
            ),
            (
                "UPDATE users SET status = 'active' WHERE name LIKE '%admin%'",
                "UPDATE users SET status = S WHERE name LIKE S",
            ),
        ];

        for (input, expected) in test_cases {
            let result = analyzer.normalize_query(input);
            assert_eq!(result, expected);
        }
    }

    #[test]
    fn test_normalize_query_whitespace() {
        let analyzer = QueryAnalyzer::new();

        let test_cases = vec![
            ("SELECT   *   FROM    users", "SELECT * FROM users"),
            (
                "  UPDATE  users  SET  name='John'  ",
                "UPDATE users SET name=S",
            ),
            (
                "SELECT\n*\nFROM\nusers\nWHERE\nid=1",
                "SELECT * FROM users WHERE id=N",
            ),
        ];

        for (input, expected) in test_cases {
            let result = analyzer.normalize_query(input);
            assert_eq!(result, expected);
        }
    }

    #[test]
    fn test_normalize_query_complex() {
        let analyzer = QueryAnalyzer::new();

        let input = "SELECT u.name, u.email FROM users u WHERE u.age > 25 AND u.status = 'active' AND u.id = $1";
        let expected =
            "SELECT u.name, u.email FROM users u WHERE u.age > N AND u.status = S AND u.id = ?";

        let result = analyzer.normalize_query(input);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_analyze_empty_entries() {
        let analyzer = QueryAnalyzer::new();
        let result = analyzer.analyze(&[]).unwrap();

        assert_eq!(result.total_queries, 0);
        assert_eq!(result.total_duration, 0.0);
        assert_eq!(result.average_duration, 0.0);
        assert_eq!(result.error_count, 0);
        assert_eq!(result.connection_count, 0);
        assert!(result.slowest_queries.is_empty());
        assert!(result.most_frequent_queries.is_empty());
        assert!(result.query_types.is_empty());
    }

    #[test]
    fn test_analyze_single_query() {
        let analyzer = QueryAnalyzer::new();
        let entries = vec![create_test_entry(
            Utc::now(),
            LogLevel::Statement,
            Some("SELECT * FROM users".to_string()),
            Some(100.0),
            None,
            None,
            None,
        )];

        let result = analyzer.analyze(&entries).unwrap();

        assert_eq!(result.total_queries, 1);
        assert_eq!(result.total_duration, 100.0);
        assert_eq!(result.average_duration, 100.0);
        assert_eq!(result.error_count, 0);
        assert_eq!(result.connection_count, 0);
        assert_eq!(result.query_types.get("SELECT"), Some(&1));
        assert_eq!(result.most_frequent_queries.len(), 1);
        assert_eq!(result.most_frequent_queries[0].0, "SELECT * FROM users");
        assert_eq!(result.most_frequent_queries[0].1, 1);
    }

    #[test]
    fn test_analyze_diverse_entries() {
        let analyzer = QueryAnalyzer::new();
        let entries = create_diverse_test_entries();

        let result = analyzer.analyze(&entries).unwrap();

        // Should have 13 query entries (excluding errors and connection logs)
        assert_eq!(result.total_queries, 13);

        // Total duration should be sum of all query durations
        let expected_total = 50.0
            + 25.0
            + 150.0
            + 10.0
            + 15.0
            + 30.0
            + 75.0
            + 2000.0
            + 100.0
            + 1.0
            + 2.0
            + 45.0
            + 55.0;
        assert_eq!(result.total_duration, expected_total);

        // Average duration
        assert_eq!(result.average_duration, expected_total / 13.0);

        // Error count should be 2
        assert_eq!(result.error_count, 2);

        // Query type distribution
        assert_eq!(result.query_types.get("SELECT"), Some(&5)); // 3 SELECT queries (including duplicates)
        assert_eq!(result.query_types.get("INSERT"), Some(&2));
        assert_eq!(result.query_types.get("UPDATE"), Some(&1));
        assert_eq!(result.query_types.get("DELETE"), Some(&1));
        assert_eq!(result.query_types.get("DDL"), Some(&2));
        assert_eq!(result.query_types.get("OTHER"), Some(&2));
    }

    #[test]
    fn test_analyze_slow_queries() {
        let analyzer = QueryAnalyzer::with_settings(100.0, 5, 10); // 100ms threshold
        let entries = create_diverse_test_entries();

        let result = analyzer.analyze(&entries).unwrap();

        // Should identify slow queries (> 100ms)
        assert!(!result.slowest_queries.is_empty());

        // Should be sorted by duration (descending)
        let mut prev_duration = f64::INFINITY;
        for (_, duration) in &result.slowest_queries {
            assert!(*duration <= prev_duration);
            assert!(*duration > 100.0); // Above threshold
            prev_duration = *duration;
        }

        // Should include the CREATE INDEX query (2000ms)
        assert!(result
            .slowest_queries
            .iter()
            .any(|(query, duration)| { query.contains("CREATE INDEX") && *duration == 2000.0 }));
    }

    #[test]
    fn test_analyze_frequent_queries() {
        let analyzer = QueryAnalyzer::new();
        let entries = create_diverse_test_entries();

        let result = analyzer.analyze(&entries).unwrap();

        // Should identify most frequent queries
        assert!(!result.most_frequent_queries.is_empty());

        // Should be sorted by frequency (descending)
        let mut prev_count = u64::MAX;
        for (_, count) in &result.most_frequent_queries {
            assert!(*count <= prev_count);
            prev_count = *count;
        }

        // The duplicate SELECT query should be most frequent (appears 3 times)
        let most_frequent = &result.most_frequent_queries[0];
        assert_eq!(most_frequent.1, 3); // Count should be 3
        assert!(most_frequent
            .0
            .contains("SELECT * FROM users WHERE active = ?"));
    }

    #[test]
    fn test_calculate_metrics() {
        let analyzer = QueryAnalyzer::new();
        let durations = vec![
            10.0, 20.0, 30.0, 40.0, 50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0,
        ];

        let metrics = analyzer.calculate_metrics(&durations);

        assert_eq!(metrics.total_queries, 10);
        assert_eq!(metrics.total_duration, 3950.0);
        assert_eq!(metrics.average_duration, 395.0);
        assert_eq!(metrics.min_duration, 10.0);
        assert_eq!(metrics.max_duration, 2000.0);

        // P95 should be around the 95th percentile
        assert!(metrics.p95_duration >= 1000.0);

        // P99 should be around the 99th percentile
        assert!(metrics.p99_duration >= 1000.0);
    }

    #[test]
    fn test_calculate_metrics_empty() {
        let analyzer = QueryAnalyzer::new();
        let durations = vec![];

        let metrics = analyzer.calculate_metrics(&durations);

        assert_eq!(metrics.total_queries, 0);
        assert_eq!(metrics.total_duration, 0.0);
        assert_eq!(metrics.average_duration, 0.0);
        assert_eq!(metrics.min_duration, 0.0);
        assert_eq!(metrics.max_duration, 0.0);
        assert_eq!(metrics.p95_duration, 0.0);
        assert_eq!(metrics.p99_duration, 0.0);
    }

    #[test]
    fn test_find_slow_queries() {
        let analyzer = QueryAnalyzer::new();
        let entries = create_diverse_test_entries();

        let slow_queries = analyzer.find_slow_queries(&entries, 100.0).unwrap();

        // Should find queries with duration > 100ms
        assert!(!slow_queries.is_empty());

        for entry in slow_queries {
            assert!(entry.duration.unwrap_or(0.0) > 100.0);
            assert!(entry.is_query());
        }
    }

    #[test]
    fn test_get_query_type_distribution() {
        let analyzer = QueryAnalyzer::new();
        let entries = create_diverse_test_entries();

        let distribution = analyzer.get_query_type_distribution(&entries);

        assert_eq!(distribution.get(&QueryType::Select), Some(&5));
        assert_eq!(distribution.get(&QueryType::Insert), Some(&2));
        assert_eq!(distribution.get(&QueryType::Update), Some(&1));
        assert_eq!(distribution.get(&QueryType::Delete), Some(&1));
        assert_eq!(distribution.get(&QueryType::DDL), Some(&2));
        assert_eq!(distribution.get(&QueryType::Other), Some(&2));
    }

    #[test]
    fn test_calculate_error_rate() {
        let analyzer = QueryAnalyzer::new();
        let entries = create_diverse_test_entries();

        let error_rate = analyzer.calculate_error_rate(&entries);

        // 2 errors out of 16 total entries = 0.125
        assert_eq!(error_rate, 2.0 / 16.0);
    }

    #[test]
    fn test_calculate_error_rate_no_entries() {
        let analyzer = QueryAnalyzer::new();
        let entries = vec![];

        let error_rate = analyzer.calculate_error_rate(&entries);
        assert_eq!(error_rate, 0.0);
    }

    #[test]
    fn test_calculate_error_rate_no_errors() {
        let analyzer = QueryAnalyzer::new();
        let entries = vec![create_test_entry(
            Utc::now(),
            LogLevel::Statement,
            Some("SELECT * FROM users".to_string()),
            Some(100.0),
            None,
            None,
            None,
        )];

        let error_rate = analyzer.calculate_error_rate(&entries);
        assert_eq!(error_rate, 0.0);
    }

    #[test]
    fn test_hourly_statistics() {
        let analyzer = QueryAnalyzer::new();

        // Create entries across different hours
        let base_time = Utc.with_ymd_and_hms(2024, 8, 15, 10, 0, 0).unwrap();
        let entries = vec![
            create_test_entry(
                base_time,
                LogLevel::Statement,
                Some("SELECT * FROM users".to_string()),
                Some(100.0),
                None,
                None,
                None,
            ),
            create_test_entry(
                base_time + chrono::Duration::hours(1),
                LogLevel::Statement,
                Some("SELECT * FROM orders".to_string()),
                Some(200.0),
                None,
                None,
                None,
            ),
            create_test_entry(
                base_time + chrono::Duration::hours(1) + chrono::Duration::minutes(30),
                LogLevel::Statement,
                Some("INSERT INTO logs VALUES (1)".to_string()),
                Some(50.0),
                None,
                None,
                None,
            ),
        ];

        let result = analyzer.analyze(&entries).unwrap();

        // Should have processed queries from different hours
        assert_eq!(result.total_queries, 3);
        assert_eq!(result.total_duration, 350.0);
    }

    #[test]
    fn test_performance_with_large_dataset() {
        let analyzer = QueryAnalyzer::new();

        // Create a large number of entries
        let mut entries = Vec::new();
        let base_time = Utc::now();

        for i in 0..1000 {
            entries.push(create_test_entry(
                base_time + chrono::Duration::seconds(i),
                LogLevel::Statement,
                Some(format!("SELECT * FROM table_{}", i % 10)),
                Some((i % 100) as f64),
                Some(&format!("{}", 12345 + i)),
                Some("postgres"),
                Some("testdb"),
            ));
        }

        let start = std::time::Instant::now();
        let result = analyzer.analyze(&entries);
        let duration = start.elapsed();

        assert!(result.is_ok());
        let analysis = result.unwrap();
        assert_eq!(analysis.total_queries, 1000);

        // Should complete within reasonable time
        assert!(
            duration.as_millis() < 1000,
            "Analysis took too long: {:?}",
            duration
        );
    }

    #[test]
    fn test_memory_usage_with_many_unique_queries() {
        let analyzer = QueryAnalyzer::new();

        // Create many unique queries
        let mut entries = Vec::new();
        let base_time = Utc::now();

        for i in 0..100 {
            entries.push(create_test_entry(
                base_time + chrono::Duration::seconds(i),
                LogLevel::Statement,
                Some(format!("SELECT * FROM users WHERE id = {}", i)),
                Some(10.0),
                Some(&format!("{}", 12345 + i)),
                Some("postgres"),
                Some("testdb"),
            ));
        }

        let result = analyzer.analyze(&entries).unwrap();

        // Should handle many unique queries without memory issues
        assert_eq!(result.total_queries, 100);
        assert_eq!(result.most_frequent_queries.len(), 1); // All unique
    }
}

#[cfg(test)]
mod query_type_tests {
    use super::*;

    #[test]
    fn test_query_type_display() {
        assert_eq!(QueryType::Select.to_string(), "SELECT");
        assert_eq!(QueryType::Insert.to_string(), "INSERT");
        assert_eq!(QueryType::Update.to_string(), "UPDATE");
        assert_eq!(QueryType::Delete.to_string(), "DELETE");
        assert_eq!(QueryType::DDL.to_string(), "DDL");
        assert_eq!(QueryType::Other.to_string(), "OTHER");
    }

    #[test]
    fn test_query_type_equality() {
        assert_eq!(QueryType::Select, QueryType::Select);
        assert_ne!(QueryType::Select, QueryType::Insert);
    }

    #[test]
    fn test_query_type_hash() {
        let mut map = HashMap::new();
        map.insert(QueryType::Select, 10);
        map.insert(QueryType::Insert, 5);

        assert_eq!(map.get(&QueryType::Select), Some(&10));
        assert_eq!(map.get(&QueryType::Insert), Some(&5));
        assert_eq!(map.get(&QueryType::Update), None);
    }
}

#[cfg(test)]
mod query_metrics_tests {
    use super::*;

    #[test]
    fn test_query_metrics_default() {
        let metrics = QueryMetrics::default();

        assert_eq!(metrics.min_duration, 0.0);
        assert_eq!(metrics.max_duration, 0.0);
        assert_eq!(metrics.average_duration, 0.0);
        assert_eq!(metrics.p95_duration, 0.0);
        assert_eq!(metrics.p99_duration, 0.0);
        assert_eq!(metrics.total_queries, 0);
        assert_eq!(metrics.total_duration, 0.0);
    }

    #[test]
    fn test_query_metrics_serialization() {
        let metrics = QueryMetrics {
            min_duration: 10.0,
            max_duration: 1000.0,
            average_duration: 100.0,
            p95_duration: 500.0,
            p99_duration: 800.0,
            total_queries: 100,
            total_duration: 10000.0,
        };

        // Test serialization to JSON
        let json = serde_json::to_string(&metrics).unwrap();
        assert!(json.contains("\"min_duration\":10.0"));
        assert!(json.contains("\"total_queries\":100"));

        // Test deserialization from JSON
        let deserialized: QueryMetrics = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.min_duration, 10.0);
        assert_eq!(deserialized.total_queries, 100);
    }
}

#[cfg(test)]
mod property_based_analytics_tests {
    use super::*;

    /// Property: Analysis results should be consistent regardless of entry order
    #[test]
    fn property_analysis_order_independent() {
        let analyzer = QueryAnalyzer::new();
        let mut entries = create_diverse_test_entries();

        // Analyze original order
        let result1 = analyzer.analyze(&entries).unwrap();

        // Reverse the order
        entries.reverse();
        let result2 = analyzer.analyze(&entries).unwrap();

        // Results should be the same
        assert_eq!(result1.total_queries, result2.total_queries);
        assert_eq!(result1.total_duration, result2.total_duration);
        assert_eq!(result1.average_duration, result2.average_duration);
        assert_eq!(result1.error_count, result2.error_count);
        assert_eq!(result1.query_types, result2.query_types);
    }

    /// Property: Query normalization should reduce the number of unique queries
    #[test]
    fn property_normalization_reduces_uniqueness() {
        let analyzer = QueryAnalyzer::new();

        let similar_queries = vec![
            "SELECT * FROM users WHERE id = 1",
            "SELECT * FROM users WHERE id = 2",
            "SELECT * FROM users WHERE id = 999",
            "SELECT * FROM users WHERE id = $1",
            "SELECT * FROM users WHERE id = $2",
        ];

        let mut unique_original = std::collections::HashSet::new();
        let mut unique_normalized = std::collections::HashSet::new();

        for query in similar_queries {
            unique_original.insert(query.to_string());
            unique_normalized.insert(analyzer.normalize_query(query));
        }

        // Normalization should reduce uniqueness
        assert!(unique_normalized.len() < unique_original.len());

        // All normalized queries should be the same
        assert_eq!(unique_normalized.len(), 2);
        assert!(unique_normalized.contains("SELECT * FROM users WHERE id = ?"));
    }
}
