//! Integration tests for pg-logstats Phase 1 implementation
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

fn baseline_slow_query_diff_content() -> &'static str {
    r#"2024-01-15 09:00:00.000 UTC [2001] app@appdb api: LOG: statement: SELECT * FROM users WHERE id = 1;
2024-01-15 09:00:00.020 UTC [2001] app@appdb api: LOG: duration: 20.000 ms
2024-01-15 09:00:01.000 UTC [2002] app@appdb api: LOG: statement: SELECT * FROM users WHERE id = 2;
2024-01-15 09:00:01.030 UTC [2002] app@appdb api: LOG: duration: 30.000 ms"#
}

fn target_slow_query_diff_content() -> &'static str {
    r#"2024-01-15 10:00:00.000 UTC [3001] app@appdb api: LOG: statement: SELECT * FROM users WHERE id = 3;
2024-01-15 10:00:00.100 UTC [3001] app@appdb api: LOG: duration: 100.000 ms
2024-01-15 10:00:01.000 UTC [3002] app@appdb api: LOG: statement: SELECT * FROM users WHERE id = 4;
2024-01-15 10:00:01.150 UTC [3002] app@appdb api: LOG: duration: 150.000 ms
2024-01-15 10:00:02.000 UTC [3003] app@appdb api: LOG: statement: SELECT * FROM orders WHERE id = 1;
2024-01-15 10:00:02.200 UTC [3003] app@appdb api: LOG: duration: 200.000 ms"#
}

fn finding_id_for_users_select() -> &'static str {
    "query_family:queryid=|db=testdb|user=testuser|app=psql|sql=SELECT * FROM users WHERE id = ?"
}

fn repo_fixture(path: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(path)
}

#[test]
fn test_cli_help() {
    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "A PostgreSQL log investigation CLI",
        ))
        .stdout(predicate::str::contains("--output-format"))
        .stdout(predicate::str::contains("top"))
        .stdout(predicate::str::contains("slow-queries"))
        .stdout(predicate::str::contains("suggest-sql"))
        .stdout(predicate::str::contains("Perl module JSON::XS").not())
        .stdout(predicate::str::contains("out.html").not());
}

#[test]
fn test_cli_version() {
    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("pg-logstats"));
}

#[test]
fn test_single_log_file_text_output() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "test.log", sample_log_content());

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.arg("--output-format")
        .arg("text")
        .arg("--quiet")
        .arg("top")
        .arg("query-families")
        .arg("--limit")
        .arg("3")
        .arg(log_file.to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("Findings"))
        .stdout(predicate::str::contains("Schema Version: 1"))
        .stdout(predicate::str::contains("#1 [query_family:"))
        .stdout(predicate::str::contains("SELECT * FROM users WHERE id = ?"))
        .stdout(predicate::str::contains(
            "INSERT INTO users (name, email) VALUES (?, ?)",
        ))
        .stdout(predicate::str::contains(
            "UPDATE users SET last_login = NOW() WHERE id = ?",
        ));
}

#[test]
fn test_single_log_file_json_output() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "test.log", sample_log_content());

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.arg("--output-format")
        .arg("json")
        .arg("--quiet")
        .arg("top")
        .arg("query-families")
        .arg("--limit")
        .arg("3")
        .arg(log_file.to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"schema_version\": 1"))
        .stdout(predicate::str::contains("\"kind\": \"query_family\""))
        .stdout(predicate::str::contains("\"total_duration_ms\": 15.234"))
        .stdout(predicate::str::contains("\"execution_count\": 1"));
}

#[test]
fn test_log_directory_processing() {
    let temp_dir = TempDir::new().unwrap();
    create_test_log_file(temp_dir.path(), "postgres.log", sample_log_content());
    create_test_log_file(temp_dir.path(), "queries.log", sample_log_content());

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.arg("--output-format")
        .arg("json")
        .arg("--quiet")
        .arg("top")
        .arg("query-families")
        .arg("--log-dir")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--limit")
        .arg("3")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"execution_count\": 2")); // top finding appears twice across two files
}

#[test]
fn test_sample_size_limiting() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "test.log", sample_log_content());

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.arg("--output-format")
        .arg("json")
        .arg("--quiet")
        .arg("top")
        .arg("query-families")
        .arg("--sample-size")
        .arg("5")
        .arg("--limit")
        .arg("5")
        .arg(log_file.to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"rank\": 1"))
        .stdout(predicate::str::contains("\"rank\": 2"))
        .stdout(predicate::str::contains("\"rank\": 3"))
        .stdout(predicate::str::contains("\"partial_correlation\""));
}

#[test]
fn test_output_to_file() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "test.log", sample_log_content());
    let output_file = temp_dir.path().join("results.json");

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.arg("--output-format")
        .arg("json")
        .arg("--outfile")
        .arg(output_file.to_str().unwrap())
        .arg("--quiet")
        .arg("top")
        .arg("query-families")
        .arg("--limit")
        .arg("3")
        .arg(log_file.to_str().unwrap())
        .assert()
        .success();

    // Verify output file was created and contains expected content
    assert!(output_file.exists());
    let content = fs::read_to_string(&output_file).unwrap();
    assert!(content.contains("\"schema_version\": 1"));
    assert!(content.contains("\"total_duration_ms\": 15.234"));
    assert!(content.contains("\"kind\": \"query_family\""));
}

#[test]
fn test_top_query_families_json_output() {
    let temp_dir = TempDir::new().unwrap();
    create_test_log_file(temp_dir.path(), "test.log", sample_log_content());

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.arg("--output-format")
        .arg("json")
        .arg("--quiet")
        .arg("top")
        .arg("query-families")
        .arg("--log-dir")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--limit")
        .arg("2")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"schema_version\": 1"))
        .stdout(predicate::str::contains("\"kind\": \"query_family\""))
        .stdout(predicate::str::contains("\"rank\": 1"))
        .stdout(predicate::str::contains("\"rank\": 2"))
        .stdout(predicate::str::contains("\"rank\": 3").not())
        .stdout(predicate::str::contains("\"correlated_duration\""))
        .stdout(predicate::str::contains("\"total_duration_ms\": 15.234"));
}

#[test]
fn test_top_query_families_text_output() {
    let temp_dir = TempDir::new().unwrap();
    create_test_log_file(temp_dir.path(), "test.log", sample_log_content());

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.arg("--quiet")
        .arg("top")
        .arg("query-families")
        .arg("--log-dir")
        .arg(temp_dir.path().to_str().unwrap())
        .arg("--limit")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Findings"))
        .stdout(predicate::str::contains("Schema Version: 1"))
        .stdout(predicate::str::contains("#1 [query_family:"))
        .stdout(predicate::str::contains(
            "Query family with high total runtime",
        ))
        .stdout(predicate::str::contains("SELECT * FROM users WHERE id = ?"))
        .stdout(predicate::str::contains("#2 [query_family:").not());
}

#[test]
fn test_slow_queries_diff_json_output() {
    let temp_dir = TempDir::new().unwrap();
    let baseline = create_test_log_file(
        temp_dir.path(),
        "baseline.log",
        baseline_slow_query_diff_content(),
    );
    let target = create_test_log_file(
        temp_dir.path(),
        "target.log",
        target_slow_query_diff_content(),
    );

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.arg("--output-format")
        .arg("json")
        .arg("--quiet")
        .arg("slow-queries")
        .arg("diff")
        .arg("--baseline")
        .arg(baseline.to_str().unwrap())
        .arg("--target")
        .arg(target.to_str().unwrap())
        .arg("--limit")
        .arg("2")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"kind\": \"slow_query_regression\"",
        ))
        .stdout(predicate::str::contains("\"p95_regressed\""))
        .stdout(predicate::str::contains("\"absent_in_baseline\""))
        .stdout(predicate::str::contains("\"baseline\""))
        .stdout(predicate::str::contains("\"target\""))
        .stdout(predicate::str::contains("\"delta\""))
        .stdout(predicate::str::contains("\"rank\": 1"))
        .stdout(predicate::str::contains("\"rank\": 2"))
        .stdout(predicate::str::contains("\"rank\": 3").not());
}

#[test]
fn test_slow_queries_diff_thresholds_filter_results() {
    let temp_dir = TempDir::new().unwrap();
    let baseline = create_test_log_file(
        temp_dir.path(),
        "baseline.log",
        baseline_slow_query_diff_content(),
    );
    let target = create_test_log_file(
        temp_dir.path(),
        "target.log",
        target_slow_query_diff_content(),
    );

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.arg("--output-format")
        .arg("json")
        .arg("--quiet")
        .arg("slow-queries")
        .arg("diff")
        .arg("--baseline")
        .arg(baseline.to_str().unwrap())
        .arg("--target")
        .arg(target.to_str().unwrap())
        .arg("--min-target-count")
        .arg("3")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"findings\": []"));
}

#[test]
fn test_suggest_sql_by_rank_from_findings_file() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "test.log", sample_log_content());
    let findings_file = temp_dir.path().join("findings.json");

    Command::cargo_bin("pg-logstats")
        .unwrap()
        .arg("--output-format")
        .arg("json")
        .arg("--outfile")
        .arg(findings_file.to_str().unwrap())
        .arg("--quiet")
        .arg("top")
        .arg("query-families")
        .arg("--limit")
        .arg("3")
        .arg(log_file.to_str().unwrap())
        .assert()
        .success();

    Command::cargo_bin("pg-logstats")
        .unwrap()
        .arg("--output-format")
        .arg("json")
        .arg("--quiet")
        .arg("suggest-sql")
        .arg("--findings-file")
        .arg(findings_file.to_str().unwrap())
        .arg("--rank")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"rank\": 1"))
        .stdout(predicate::str::contains("\"next_sql\""))
        .stdout(predicate::str::contains("pg_stat_statements"))
        .stdout(predicate::str::contains("pg_stat_activity"));
}

#[test]
fn test_suggest_sql_by_finding_id() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "test.log", sample_log_content());
    let findings_file = temp_dir.path().join("findings.json");

    Command::cargo_bin("pg-logstats")
        .unwrap()
        .arg("--output-format")
        .arg("json")
        .arg("--outfile")
        .arg(findings_file.to_str().unwrap())
        .arg("--quiet")
        .arg("top")
        .arg("query-families")
        .arg("--limit")
        .arg("3")
        .arg(log_file.to_str().unwrap())
        .assert()
        .success();

    Command::cargo_bin("pg-logstats")
        .unwrap()
        .arg("--quiet")
        .arg("suggest-sql")
        .arg("--findings-file")
        .arg(findings_file.to_str().unwrap())
        .arg("--finding-id")
        .arg(finding_id_for_users_select())
        .assert()
        .success()
        .stdout(predicate::str::contains(finding_id_for_users_select()))
        .stdout(predicate::str::contains("pg_stat_statements"))
        .stdout(predicate::str::contains("pg_stat_activity"));
}

#[test]
fn test_empty_log_file() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "empty.log", "");

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.arg(log_file.to_str().unwrap())
        .arg("--quiet")
        .arg("top")
        .arg("query-families")
        .assert()
        .failure(); // Should exit with error code for no entries
}

#[test]
fn test_nonexistent_log_file() {
    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.arg("--quiet")
        .arg("top")
        .arg("query-families")
        .arg("nonexistent.log")
        .assert()
        .failure(); // Should exit with error code
}

#[test]
fn test_nonexistent_log_directory() {
    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.arg("--quiet")
        .arg("top")
        .arg("query-families")
        .arg("--log-dir")
        .arg("/nonexistent/directory")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Log directory does not exist"));
}

#[test]
fn test_invalid_sample_size() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "test.log", sample_log_content());

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.arg("--quiet")
        .arg("top")
        .arg("query-families")
        .arg("--sample-size")
        .arg("0")
        .arg(log_file.to_str().unwrap())
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Sample size must be greater than 0",
        ));
}

#[test]
fn test_malformed_log_lines() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "malformed.log", malformed_log_content());

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.arg("--output-format")
        .arg("json")
        .arg("--quiet")
        .arg("top")
        .arg("query-families")
        .arg(log_file.to_str().unwrap())
        .assert()
        .success() // Should succeed but with fewer parsed entries
        .stdout(predicate::str::contains("\"rank\": 1"))
        .stdout(predicate::str::contains("\"rank\": 2").not()); // Only 1 correlated execution
}

#[test]
fn test_progress_bar_disabled_in_quiet_mode() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "test.log", sample_log_content());

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.arg("--quiet")
        .arg("top")
        .arg("query-families")
        .arg(log_file.to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("Processing").not()); // No progress messages
}

#[test]
fn test_progress_bar_enabled_by_default() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "test.log", sample_log_content());

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.arg("top")
        .arg("query-families")
        .arg(log_file.to_str().unwrap())
        .timeout(std::time::Duration::from_secs(10))
        .assert()
        .success();
    // Note: Progress bar output is complex to test in integration tests
    // This mainly verifies the command completes successfully
}

#[test]
fn test_global_flags_work_after_subcommand() {
    let fixture = repo_fixture("examples/logs/sample_stderr.log");

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.arg("top")
        .arg("query-families")
        .arg("--quiet")
        .arg("--output-format")
        .arg("json")
        .arg(fixture.to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"kind\": \"query_family\""));
}

#[test]
fn test_checked_in_top_query_families_fixture_smoke() {
    let fixture = repo_fixture("examples/logs/sample_stderr.log");

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.arg("top")
        .arg("query-families")
        .arg("--quiet")
        .arg(fixture.to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("Findings"))
        .stdout(predicate::str::contains("SELECT * FROM users WHERE id = ?"))
        .stdout(predicate::str::contains("44.000 ms total runtime"));
}

#[test]
fn test_checked_in_slow_query_diff_fixture_smoke() {
    let baseline = repo_fixture("examples/logs/diff_baseline.log");
    let target = repo_fixture("examples/logs/diff_target.log");

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.arg("slow-queries")
        .arg("diff")
        .arg("--quiet")
        .arg("--output-format")
        .arg("json")
        .arg("--baseline")
        .arg(baseline.to_str().unwrap())
        .arg("--target")
        .arg(target.to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"kind\": \"slow_query_regression\"",
        ))
        .stdout(predicate::str::contains("\"p95_regressed\""));
}

#[test]
fn test_checked_in_suggest_sql_happy_path() {
    let fixture = repo_fixture("examples/logs/sample_stderr.log");
    let temp_dir = TempDir::new().unwrap();
    let findings_file = temp_dir.path().join("findings.json");

    Command::cargo_bin("pg-logstats")
        .unwrap()
        .arg("top")
        .arg("query-families")
        .arg("--quiet")
        .arg("--output-format")
        .arg("json")
        .arg("--outfile")
        .arg(findings_file.to_str().unwrap())
        .arg(fixture.to_str().unwrap())
        .assert()
        .success();

    Command::cargo_bin("pg-logstats")
        .unwrap()
        .arg("suggest-sql")
        .arg("--quiet")
        .arg("--findings-file")
        .arg(findings_file.to_str().unwrap())
        .arg("--rank")
        .arg("1")
        .assert()
        .success()
        .stdout(predicate::str::contains("pg_stat_statements"))
        .stdout(predicate::str::contains("SELECT * FROM users WHERE id = ?"));
}

#[test]
fn test_large_file_processing() {
    let temp_dir = TempDir::new().unwrap();
    let large_content = large_log_content(1000); // 1000 log entries
    let log_file = create_test_log_file(temp_dir.path(), "large.log", &large_content);

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.arg("--output-format")
        .arg("json")
        .arg("--quiet")
        .arg("top")
        .arg("query-families")
        .arg(log_file.to_str().unwrap())
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success()
        .stdout(predicate::str::contains("\"execution_count\": 1000"));
}

#[test]
fn test_multiple_log_files() {
    let temp_dir = TempDir::new().unwrap();
    let log_file1 = create_test_log_file(temp_dir.path(), "test1.log", sample_log_content());
    let log_file2 = create_test_log_file(temp_dir.path(), "test2.log", sample_log_content());

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.arg("--output-format")
        .arg("json")
        .arg("--quiet")
        .arg("top")
        .arg("query-families")
        .arg(log_file1.to_str().unwrap())
        .arg(log_file2.to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"execution_count\": 2")); // top finding appears twice across two files
}

#[test]
fn test_mixed_valid_invalid_files() {
    let temp_dir = TempDir::new().unwrap();
    let valid_file = create_test_log_file(temp_dir.path(), "valid.log", sample_log_content());
    let invalid_file = temp_dir.path().join("nonexistent.log");

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.arg("--output-format")
        .arg("json")
        .arg("--quiet")
        .arg("top")
        .arg("query-families")
        .arg(valid_file.to_str().unwrap())
        .arg(invalid_file.to_str().unwrap())
        .assert()
        .success() // Should succeed with valid file, warn about invalid
        .stdout(predicate::str::contains("\"execution_count\": 1"));
}

#[test]
fn test_verbose_logging() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "test.log", sample_log_content());

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.env("RUST_LOG", "debug")
        .arg("--quiet")
        .arg("top")
        .arg("query-families")
        .arg(log_file.to_str().unwrap())
        .assert()
        .success()
        .stderr(predicate::str::contains("DEBUG"))
        .stderr(predicate::str::contains("Initializing stderr parser"));
}

#[test]
fn test_json_output_structure() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "test.log", sample_log_content());

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    let output = cmd
        .arg("--output-format")
        .arg("json")
        .arg("--quiet")
        .arg("top")
        .arg("query-families")
        .arg("--limit")
        .arg("3")
        .arg(log_file.to_str().unwrap())
        .output()
        .unwrap();

    assert!(output.status.success());
    let json_str = String::from_utf8(output.stdout).unwrap();

    // Parse JSON to verify structure
    let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

    assert!(json["metadata"].is_object());
    assert!(json["findings"].is_array());

    assert!(json["metadata"]["tool_version"].is_string());
    assert!(json["metadata"]["total_log_entries"].is_number());
    assert!(json["findings"][0]["kind"].is_string());
}

#[test]
fn test_performance_with_sample_size() {
    let temp_dir = TempDir::new().unwrap();
    let large_content = large_log_content(10000); // 10,000 log entries
    let log_file = create_test_log_file(temp_dir.path(), "huge.log", &large_content);

    let start = std::time::Instant::now();

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.arg("--output-format")
        .arg("json")
        .arg("--quiet")
        .arg("top")
        .arg("query-families")
        .arg("--sample-size")
        .arg("100") // Limit to first 100 lines
        .arg(log_file.to_str().unwrap())
        .timeout(std::time::Duration::from_secs(10))
        .assert()
        .success()
        .stdout(predicate::str::contains("\"execution_count\": 100"));

    let elapsed = start.elapsed();
    assert!(elapsed < std::time::Duration::from_secs(5)); // Should be fast with sampling
}

#[cfg(test)]
mod docker_tests {
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

        let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
        cmd.arg("--quiet")
            .arg("top")
            .arg("query-families")
            .arg(log_file.to_str().unwrap())
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
        let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
        cmd.arg("--output-format")
            .arg("json")
            .arg("--quiet")
            .arg("top")
            .arg("query-families")
            .arg(log_file.to_str().unwrap())
            .timeout(std::time::Duration::from_secs(15))
            .assert()
            .success()
            .stdout(predicate::str::contains("\"execution_count\": 1000"));
    }
}
