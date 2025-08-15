//! Integration tests for pg-loggrep Phase 1 implementation
//!
//! These tests verify the complete workflow from CLI arguments to output generation.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

/// Helper function to create a test log file with sample data
fn create_test_log_file(dir: &Path, filename: &str, content: &str) -> std::path::PathBuf {
    let file_path = dir.join(filename);
    fs::write(&file_path, content).expect("Failed to write test log file");
    file_path
}

/// Helper function to create sample PostgreSQL log content
fn sample_log_content() -> &'static str {
    r#"2024-01-15 10:00:00.123 UTC [1234] testuser@testdb psql: LOG: statement: SELECT * FROM users WHERE id = 1;
2024-01-15 10:00:01.456 UTC [1234] testuser@testdb psql: LOG: duration: 15.234 ms
2024-01-15 10:00:02.789 UTC [1235] admin@testdb pgAdmin: LOG: statement: INSERT INTO users (name, email) VALUES ('John Doe', 'john@example.com');
2024-01-15 10:00:03.012 UTC [1235] admin@testdb pgAdmin: LOG: duration: 8.567 ms
2024-01-15 10:00:04.345 UTC [1236] testuser@testdb psql: LOG: statement: UPDATE users SET last_login = NOW() WHERE id = 1;
2024-01-15 10:00:05.678 UTC [1236] testuser@testdb psql: LOG: duration: 12.890 ms
2024-01-15 10:00:06.901 UTC [1237] testuser@testdb psql: ERROR: relation "nonexistent_table" does not exist
2024-01-15 10:00:07.234 UTC [1238] admin@testdb pgAdmin: LOG: statement: SELECT COUNT(*) FROM users;
2024-01-15 10:00:08.567 UTC [1238] admin@testdb pgAdmin: LOG: duration: 5.123 ms"#
}

/// Helper function to create malformed log content for error testing
fn malformed_log_content() -> &'static str {
    r#"This is not a valid log line
2024-01-15 10:00:00.123 UTC [1234] testuser@testdb psql: LOG: statement: SELECT * FROM users;
Another invalid line without proper format
2024-01-15 10:00:01.456 UTC [1234] testuser@testdb psql: LOG: duration: 15.234 ms
Yet another malformed line"#
}

/// Helper function to create large log content for performance testing
fn large_log_content(num_entries: usize) -> String {
    let base_entry = "2024-01-15 10:00:00.123 UTC [1234] testuser@testdb psql: LOG: statement: SELECT * FROM users WHERE id = 1;\n";
    base_entry.repeat(num_entries)
}

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("pg-loggrep").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("A fast PostgreSQL log analysis tool"))
        .stdout(predicate::str::contains("--log-dir"))
        .stdout(predicate::str::contains("--output-format"))
        .stdout(predicate::str::contains("--quick"))
        .stdout(predicate::str::contains("--sample-size"));
}

#[test]
fn test_cli_version() {
    let mut cmd = Command::cargo_bin("pg-loggrep").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("pg-loggrep"));
}

#[test]
fn test_single_log_file_text_output() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "test.log", sample_log_content());

    let mut cmd = Command::cargo_bin("pg-loggrep").unwrap();
    cmd.arg(log_file.to_str().unwrap())
        .arg("--output-format")
        .arg("text")
        .arg("--quiet")
        .assert()
        .success()
        .stdout(predicate::str::contains("Query Analysis Report"))
        .stdout(predicate::str::contains("Total Queries: 4"))
        .stdout(predicate::str::contains("Error Count: 1"))
        .stdout(predicate::str::contains("SELECT: 2"))
        .stdout(predicate::str::contains("INSERT: 1"))
        .stdout(predicate::str::contains("UPDATE: 1"));
}

#[test]
fn test_single_log_file_json_output() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "test.log", sample_log_content());

    let mut cmd = Command::cargo_bin("pg-loggrep").unwrap();
    cmd.arg(log_file.to_str().unwrap())
        .arg("--output-format")
        .arg("json")
        .arg("--quiet")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"total_queries\": 4"))
        .stdout(predicate::str::contains("\"error_count\": 1"))
        .stdout(predicate::str::contains("\"SELECT\": 2"))
        .stdout(predicate::str::contains("\"INSERT\": 1"))
        .stdout(predicate::str::contains("\"UPDATE\": 1"));
}

#[test]
fn test_log_directory_processing() {
    let temp_dir = TempDir::new().unwrap();
    create_test_log_file(temp_dir.path(), "postgres.log", sample_log_content());
    create_test_log_file(temp_dir.path(), "queries.log", sample_log_content());

    let mut cmd = Command::cargo_bin("pg-loggrep").unwrap();
    cmd.arg("--log-dir")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--quiet")
        .assert()
        .success()
        .stdout(predicate::str::contains("Total Queries: 8")); // 4 queries per file * 2 files
}

#[test]
fn test_sample_size_limiting() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "test.log", sample_log_content());

    let mut cmd = Command::cargo_bin("pg-loggrep").unwrap();
    cmd.arg(log_file.to_str().unwrap())
        .arg("--sample-size")
        .arg("5")
        .arg("--quiet")
        .assert()
        .success()
        .stdout(predicate::str::contains("Total Queries: 3")); // Limited by sample size
}

#[test]
fn test_output_to_file() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "test.log", sample_log_content());
    let output_file = temp_dir.path().join("results.json");

    let mut cmd = Command::cargo_bin("pg-loggrep").unwrap();
    cmd.arg(log_file.to_str().unwrap())
        .arg("--output-format")
        .arg("json")
        .arg("--outfile")
        .arg(output_file.to_str().unwrap())
        .arg("--quiet")
        .assert()
        .success();

    // Verify output file was created and contains expected content
    assert!(output_file.exists());
    let content = fs::read_to_string(&output_file).unwrap();
    assert!(content.contains("\"total_queries\": 4"));
    assert!(content.contains("\"error_count\": 1"));
}

#[test]
fn test_empty_log_file() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "empty.log", "");

    let mut cmd = Command::cargo_bin("pg-loggrep").unwrap();
    cmd.arg(log_file.to_str().unwrap())
        .arg("--quiet")
        .assert()
        .failure(); // Should exit with error code for no entries
}

#[test]
fn test_nonexistent_log_file() {
    let mut cmd = Command::cargo_bin("pg-loggrep").unwrap();
    cmd.arg("nonexistent.log")
        .arg("--quiet")
        .assert()
        .failure(); // Should exit with error code
}

#[test]
fn test_nonexistent_log_directory() {
    let mut cmd = Command::cargo_bin("pg-loggrep").unwrap();
    cmd.arg("--log-dir")
        .arg("/nonexistent/directory")
        .arg("--quiet")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Log directory does not exist"));
}

#[test]
fn test_invalid_sample_size() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "test.log", sample_log_content());

    let mut cmd = Command::cargo_bin("pg-loggrep").unwrap();
    cmd.arg(log_file.to_str().unwrap())
        .arg("--sample-size")
        .arg("0")
        .arg("--quiet")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Sample size must be greater than 0"));
}

#[test]
fn test_malformed_log_lines() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "malformed.log", malformed_log_content());

    let mut cmd = Command::cargo_bin("pg-loggrep").unwrap();
    cmd.arg(log_file.to_str().unwrap())
        .arg("--quiet")
        .assert()
        .success() // Should succeed but with fewer parsed entries
        .stdout(predicate::str::contains("Total Queries: 1")); // Only 1 valid query line
}

#[test]
fn test_progress_bar_disabled_in_quiet_mode() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "test.log", sample_log_content());

    let mut cmd = Command::cargo_bin("pg-loggrep").unwrap();
    cmd.arg(log_file.to_str().unwrap())
        .arg("--quiet")
        .assert()
        .success()
        .stdout(predicate::str::contains("Processing").not()); // No progress messages
}

#[test]
fn test_progress_bar_enabled_by_default() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "test.log", sample_log_content());

    let mut cmd = Command::cargo_bin("pg-loggrep").unwrap();
    cmd.arg(log_file.to_str().unwrap())
        .timeout(std::time::Duration::from_secs(10))
        .assert()
        .success();
    // Note: Progress bar output is complex to test in integration tests
    // This mainly verifies the command completes successfully
}

#[test]
fn test_large_file_processing() {
    let temp_dir = TempDir::new().unwrap();
    let large_content = large_log_content(1000); // 1000 log entries
    let log_file = create_test_log_file(temp_dir.path(), "large.log", &large_content);

    let mut cmd = Command::cargo_bin("pg-loggrep").unwrap();
    cmd.arg(log_file.to_str().unwrap())
        .arg("--quiet")
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success()
        .stdout(predicate::str::contains("Total Queries: 1000"));
}

#[test]
fn test_multiple_log_files() {
    let temp_dir = TempDir::new().unwrap();
    let log_file1 = create_test_log_file(temp_dir.path(), "test1.log", sample_log_content());
    let log_file2 = create_test_log_file(temp_dir.path(), "test2.log", sample_log_content());

    let mut cmd = Command::cargo_bin("pg-loggrep").unwrap();
    cmd.arg(log_file1.to_str().unwrap())
        .arg(log_file2.to_str().unwrap())
        .arg("--quiet")
        .assert()
        .success()
        .stdout(predicate::str::contains("Total Queries: 8")); // 4 queries per file * 2 files
}

#[test]
fn test_mixed_valid_invalid_files() {
    let temp_dir = TempDir::new().unwrap();
    let valid_file = create_test_log_file(temp_dir.path(), "valid.log", sample_log_content());
    let invalid_file = temp_dir.path().join("nonexistent.log");

    let mut cmd = Command::cargo_bin("pg-loggrep").unwrap();
    cmd.arg(valid_file.to_str().unwrap())
        .arg(invalid_file.to_str().unwrap())
        .arg("--quiet")
        .assert()
        .success() // Should succeed with valid file, warn about invalid
        .stdout(predicate::str::contains("Total Queries: 4"));
}

#[test]
fn test_verbose_logging() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "test.log", sample_log_content());

    let mut cmd = Command::cargo_bin("pg-loggrep").unwrap();
    cmd.env("RUST_LOG", "debug")
        .arg(log_file.to_str().unwrap())
        .arg("--quiet")
        .assert()
        .success()
        .stderr(predicate::str::contains("DEBUG"))
        .stderr(predicate::str::contains("Initializing stderr parser"));
}

#[test]
fn test_json_output_structure() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "test.log", sample_log_content());

    let mut cmd = Command::cargo_bin("pg-loggrep").unwrap();
    let output = cmd
        .arg(log_file.to_str().unwrap())
        .arg("--output-format")
        .arg("json")
        .arg("--quiet")
        .output()
        .unwrap();

    assert!(output.status.success());
    let json_str = String::from_utf8(output.stdout).unwrap();

    // Parse JSON to verify structure
    let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    assert!(json["metadata"].is_object());
    assert!(json["summary"].is_object());
    assert!(json["query_analysis"].is_object());

    assert!(json["metadata"]["tool_version"].is_string());
    assert!(json["summary"]["total_queries"].is_number());
    assert!(json["query_analysis"]["by_type"].is_object());
}

#[test]
fn test_performance_with_sample_size() {
    let temp_dir = TempDir::new().unwrap();
    let large_content = large_log_content(10000); // 10,000 log entries
    let log_file = create_test_log_file(temp_dir.path(), "huge.log", &large_content);

    let start = std::time::Instant::now();

    let mut cmd = Command::cargo_bin("pg-loggrep").unwrap();
    cmd.arg(log_file.to_str().unwrap())
        .arg("--sample-size")
        .arg("100") // Limit to first 100 lines
        .arg("--quiet")
        .timeout(std::time::Duration::from_secs(10))
        .assert()
        .success()
        .stdout(predicate::str::contains("Total Queries: 100"));

    let elapsed = start.elapsed();
    assert!(elapsed < std::time::Duration::from_secs(5)); // Should be fast with sampling
}

#[cfg(test)]
mod docker_tests {
    use super::*;

    /// Test that requires Docker to be available
    /// This test is ignored by default and can be run with: cargo test -- --ignored
    #[test]
    #[ignore]
    fn test_docker_environment() {
        // This would test the tool in a Docker container
        // Implementation depends on Docker setup requirements
        todo!("Implement Docker environment testing");
    }
}

#[cfg(test)]
mod benchmark_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn benchmark_parsing_speed() {
        let temp_dir = TempDir::new().unwrap();
        let content = large_log_content(5000); // 5,000 entries
        let log_file = create_test_log_file(temp_dir.path(), "benchmark.log", &content);

        let start = Instant::now();

        let mut cmd = Command::cargo_bin("pg-loggrep").unwrap();
        cmd.arg(log_file.to_str().unwrap())
            .arg("--quiet")
            .timeout(std::time::Duration::from_secs(30))
            .assert()
            .success();

        let elapsed = start.elapsed();
        println!("Parsed 5,000 entries in {:?}", elapsed);

        // Should complete within reasonable time (adjust based on performance requirements)
        assert!(elapsed < std::time::Duration::from_secs(10));
    }

    #[test]
    fn benchmark_memory_usage() {
        let temp_dir = TempDir::new().unwrap();
        let content = large_log_content(1000); // 1,000 entries for memory test
        let log_file = create_test_log_file(temp_dir.path(), "memory_test.log", &content);

        // This is a basic test - in production you'd use more sophisticated memory profiling
        let mut cmd = Command::cargo_bin("pg-loggrep").unwrap();
        cmd.arg(log_file.to_str().unwrap())
            .arg("--quiet")
            .timeout(std::time::Duration::from_secs(15))
            .assert()
            .success()
            .stdout(predicate::str::contains("Total Queries: 1000"));
    }
}
