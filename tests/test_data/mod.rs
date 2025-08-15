//! Test data generation utilities
//!
//! Provides functions to generate various types of test log files and expected outputs

use std::fs;
use std::path::Path;
use chrono::{DateTime, Utc, Duration};
use tempfile::TempDir;

/// Generate a comprehensive test log file with various PostgreSQL log entries
pub fn generate_comprehensive_log_file(path: &Path) -> std::io::Result<()> {
    let content = r#"2024-08-15 10:30:15.123 UTC [12345] postgres@testdb psql: LOG:  statement: SELECT * FROM users WHERE active = true;
2024-08-15 10:30:15.456 UTC [12345] postgres@testdb psql: LOG:  duration: 45.123 ms
2024-08-15 10:30:16.789 UTC [12346] admin@analytics pgbench: ERROR:  relation "missing_table" does not exist
2024-08-15 10:30:17.012 UTC [12347] postgres@testdb psql: WARNING:  there is no transaction in progress
2024-08-15 10:30:18.345 UTC [12348] postgres@testdb psql: LOG:  statement: UPDATE products SET price = $1 WHERE id = $2
2024-08-15 10:30:18.567 UTC [12348] postgres@testdb psql: LOG:  duration: 12.345 ms
2024-08-15 10:30:19.678 UTC [12349] postgres@testdb psql: LOG:  statement: SELECT u.name, p.title
    FROM users u
    JOIN posts p ON u.id = p.user_id
    WHERE u.active = true
    ORDER BY p.created_at DESC;
2024-08-15 10:30:19.890 UTC [12349] postgres@testdb psql: LOG:  duration: 156.789 ms
2024-08-15 10:30:20.123 UTC [12350] app_user@app_db web_app: LOG:  statement: INSERT INTO users (name, email) VALUES ('John Doe', 'john@example.com');
2024-08-15 10:30:20.234 UTC [12350] app_user@app_db web_app: LOG:  duration: 8.901 ms
2024-08-15 10:30:21.345 UTC [12351] cleanup_job@testdb cron: LOG:  statement: DELETE FROM sessions WHERE expires_at < NOW();
2024-08-15 10:30:21.456 UTC [12351] cleanup_job@testdb cron: LOG:  duration: 234.567 ms
2024-08-15 10:30:22.567 UTC [12352] admin@testdb psql: LOG:  statement: CREATE INDEX idx_users_email ON users(email);
2024-08-15 10:30:25.678 UTC [12352] admin@testdb psql: LOG:  duration: 3111.111 ms
2024-08-15 10:30:26.789 UTC [12353] postgres@testdb psql: LOG:  statement: BEGIN;
2024-08-15 10:30:26.790 UTC [12353] postgres@testdb psql: LOG:  duration: 0.001 ms
2024-08-15 10:30:27.123 UTC [12353] postgres@testdb psql: LOG:  statement: SELECT COUNT(*) FROM orders WHERE status = 'pending';
2024-08-15 10:30:27.234 UTC [12353] postgres@testdb psql: LOG:  duration: 111.111 ms
2024-08-15 10:30:28.345 UTC [12353] postgres@testdb psql: LOG:  statement: COMMIT;
2024-08-15 10:30:28.346 UTC [12353] postgres@testdb psql: LOG:  duration: 0.001 ms
2024-08-15 10:30:29.456 UTC [12354] postgres@testdb psql: FATAL:  database "nonexistent" does not exist
2024-08-15 10:30:30.567 UTC [12355] postgres@testdb psql: PANIC:  could not write to file "pg_wal/000000010000000000000001": No space left on device
"#;

    fs::write(path, content)
}

/// Generate a log file with edge cases (empty lines, malformed entries, etc.)
pub fn generate_edge_case_log_file(path: &Path) -> std::io::Result<()> {
    let content = r#"2024-08-15 10:30:15.123 UTC [12345] postgres@testdb psql: LOG:  statement: SELECT * FROM users;

This is not a valid log line
2024-08-15 10:30:16.456 UTC [12346] postgres@testdb psql: LOG:  duration: 45.123 ms

    Continuation line without a statement
2024-08-15 10:30:17.789 UTC [12347] postgres@testdb psql: ERROR:  syntax error at or near "SELCT"
2024-08-15 10:30:18.012 UTC [12348] postgres@testdb psql: LOG:  statement: SELECT * FROM "table-with-special-chars" WHERE name = 'O''Reilly';
2024-08-15 10:30:18.234 UTC [12348] postgres@testdb psql: LOG:  duration: 67.890 ms
2024-08-15 10:30:19.345 UTC [12349] postgres@testdb psql: LOG:  statement: SELECT * FROM users WHERE description = 'User with unicode: 测试用户';
2024-08-15 10:30:19.456 UTC [12349] postgres@testdb psql: LOG:  duration: 23.456 ms

2024-08-15 10:30:20.567 UTC [12350] postgres@testdb psql: LOG:  connection received: host=192.168.1.100 port=54321
2024-08-15 10:30:21.678 UTC [12351] postgres@testdb psql: LOG:  connection authorized: user=postgres database=testdb
"#;

    fs::write(path, content)
}

/// Generate a large log file for performance testing
pub fn generate_large_log_file(path: &Path, num_entries: usize) -> std::io::Result<()> {
    let mut content = String::new();
    let base_time = Utc::now();

    for i in 0..num_entries {
        let timestamp = base_time + Duration::seconds(i as i64);
        let process_id = 12345 + (i % 100);
        let query_type = match i % 5 {
            0 => "SELECT * FROM users WHERE id = $1",
            1 => "INSERT INTO logs (message) VALUES ($1)",
            2 => "UPDATE users SET last_seen = NOW() WHERE id = $1",
            3 => "DELETE FROM sessions WHERE expires_at < $1",
            _ => "SELECT COUNT(*) FROM orders",
        };
        let duration = (i % 1000) as f64 + 0.123;

        content.push_str(&format!(
            "{} UTC [{}] postgres@testdb psql: LOG:  statement: {};\n",
            timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
            process_id,
            query_type
        ));
        content.push_str(&format!(
            "{} UTC [{}] postgres@testdb psql: LOG:  duration: {:.3} ms\n",
            (timestamp + Duration::milliseconds(1)).format("%Y-%m-%d %H:%M:%S%.3f"),
            process_id,
            duration
        ));
    }

    fs::write(path, content)
}

/// Generate an empty log file
pub fn generate_empty_log_file(path: &Path) -> std::io::Result<()> {
    fs::write(path, "")
}

/// Generate a log file with only malformed entries
pub fn generate_malformed_log_file(path: &Path) -> std::io::Result<()> {
    let content = r#"This is not a PostgreSQL log file
Random text here
Another invalid line
2024-13-45 25:70:99 INVALID [abc] user@db app: UNKNOWN: invalid message
"#;

    fs::write(path, content)
}

/// Generate expected JSON output for comprehensive log file
pub fn generate_expected_json_output() -> serde_json::Value {
    serde_json::json!({
        "metadata": {
            "analysis_timestamp": "2024-08-15T10:30:30.000Z",
            "tool_version": "0.1.0",
            "log_files_processed": ["test.log"],
            "total_log_entries": 22
        },
        "summary": {
            "total_queries": 11,
            "total_duration_ms": 3689.456,
            "avg_duration_ms": 335.405,
            "error_count": 3,
            "connection_count": 2
        },
        "query_analysis": {
            "by_type": {
                "SELECT": 4,
                "UPDATE": 1,
                "INSERT": 1,
                "DELETE": 1,
                "DDL": 1,
                "OTHER": 3
            },
            "slowest_queries": [
                {
                    "query": "CREATE INDEX idx_users_email ON users(email)",
                    "duration_ms": 3111.111,
                    "count": 1
                },
                {
                    "query": "DELETE FROM sessions WHERE expires_at < NOW()",
                    "duration_ms": 234.567,
                    "count": 1
                },
                {
                    "query": "SELECT u.name, p.title FROM users u JOIN posts p ON u.id = p.user_id WHERE u.active = ? ORDER BY p.created_at DESC",
                    "duration_ms": 156.789,
                    "count": 1
                }
            ],
            "most_frequent": [
                {
                    "query": "SELECT * FROM users WHERE active = ?",
                    "count": 1,
                    "avg_duration_ms": 45.123
                },
                {
                    "query": "UPDATE products SET price = ? WHERE id = ?",
                    "count": 1,
                    "avg_duration_ms": 12.345
                }
            ]
        }
    })
}

/// Generate expected text output for comprehensive log file
pub fn generate_expected_text_output() -> String {
    r#"Query Analysis Report
===================
Total Queries: 11
Total Duration: 3689.46 ms
Average Duration: 335.41 ms
P95 Duration: 2800.00 ms
P99 Duration: 3000.00 ms
Error Count: 3
Connection Count: 2

Query Types:
    SELECT: 4
    UPDATE: 1
    INSERT: 1
    DELETE: 1
       DDL: 1
     OTHER: 3

Slowest Queries:
     #  Duration (ms)  Query
     1       3111.11  CREATE INDEX idx_users_email ON users(email)
     2        234.57  DELETE FROM sessions WHERE expires_at < NOW()
     3        156.79  SELECT u.name, p.title FROM users u JOIN posts p ON u.id = p.user_id WHERE u.active = ? ORDER BY p.created_at DESC

Most Frequent Queries:
     #     Count  Query
     1         1  SELECT * FROM users WHERE active = ?
     2         1  UPDATE products SET price = ? WHERE id = ?
     3         1  INSERT INTO users (name, email) VALUES (S, S)
"#.to_string()
}

/// Create a temporary directory with test log files
pub fn create_test_directory() -> std::io::Result<TempDir> {
    let temp_dir = TempDir::new()?;

    // Create various test files
    generate_comprehensive_log_file(&temp_dir.path().join("comprehensive.log"))?;
    generate_edge_case_log_file(&temp_dir.path().join("edge_cases.log"))?;
    generate_large_log_file(&temp_dir.path().join("large.log"), 1000)?;
    generate_empty_log_file(&temp_dir.path().join("empty.log"))?;
    generate_malformed_log_file(&temp_dir.path().join("malformed.log"))?;

    Ok(temp_dir)
}

/// Generate test data for property-based testing
pub fn generate_property_test_data() -> Vec<String> {
    let mut lines = Vec::new();
    let base_time = Utc::now();

    // Generate various valid log line patterns
    let patterns = vec![
        ("SELECT * FROM table_{} WHERE id = {}", "LOG"),
        ("INSERT INTO table_{} VALUES ({})", "LOG"),
        ("UPDATE table_{} SET value = {}", "LOG"),
        ("DELETE FROM table_{} WHERE id = {}", "LOG"),
        ("CREATE TABLE table_{} (id INT)", "LOG"),
        ("DROP TABLE table_{}", "LOG"),
        ("BEGIN", "LOG"),
        ("COMMIT", "LOG"),
        ("ROLLBACK", "LOG"),
    ];

    for i in 0..100 {
        let timestamp = base_time + Duration::seconds(i);
        let process_id = 12345 + (i % 10);
        let (query_template, level) = &patterns[i % patterns.len()];
        let query = query_template.replace("{}", &i.to_string());

        lines.push(format!(
            "{} UTC [{}] postgres@testdb psql: {}:  statement: {};",
            timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
            process_id,
            level,
            query
        ));

        // Add duration for some queries
        if i % 3 == 0 {
            lines.push(format!(
                "{} UTC [{}] postgres@testdb psql: LOG:  duration: {:.3} ms",
                (timestamp + Duration::milliseconds(1)).format("%Y-%m-%d %H:%M:%S%.3f"),
                process_id,
                (i % 100) as f64 + 0.123
            ));
        }
    }

    lines
}

/// Generate benchmark test data for performance testing
pub fn generate_benchmark_data(num_queries: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let base_time = Utc::now();

    for i in 0..num_queries {
        let timestamp = base_time + Duration::milliseconds(i as i64);
        let process_id = 12345 + (i % 1000);
        let duration = (i % 10000) as f64 / 10.0; // 0.0 to 999.9 ms

        // Create a mix of query types
        let query = match i % 10 {
            0..=3 => format!("SELECT * FROM table_{} WHERE id = {}", i % 100, i),
            4..=5 => format!("INSERT INTO table_{} (data) VALUES ('{}')", i % 50, i),
            6 => format!("UPDATE table_{} SET data = '{}' WHERE id = {}", i % 50, i, i % 1000),
            7 => format!("DELETE FROM table_{} WHERE id = {}", i % 50, i % 1000),
            8 => format!("CREATE INDEX idx_{}_{} ON table_{}(column_{})", i % 10, i % 5, i % 20, i % 3),
            _ => "BEGIN".to_string(),
        };

        lines.push(format!(
            "{} UTC [{}] postgres@testdb psql: LOG:  statement: {};",
            timestamp.format("%Y-%m-%d %H:%M:%S%.3f"),
            process_id,
            query
        ));

        lines.push(format!(
            "{} UTC [{}] postgres@testdb psql: LOG:  duration: {:.3} ms",
            (timestamp + Duration::milliseconds(1)).format("%Y-%m-%d %H:%M:%S%.3f"),
            process_id,
            duration
        ));
    }

    lines
}
