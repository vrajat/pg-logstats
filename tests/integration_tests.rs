//! Integration tests for pg-logstats Phase 1 implementation
//!
//! These tests verify the complete workflow from CLI arguments to output generation.

use assert_cmd::Command;
use pg_logstats::{
    inspect::{AgentInspect, AgentTargetInspect, DatabaseInspect},
    ActionKind, AppConfig, CheckStatus, FindingsPayload, InspectReportPayload, OperatingMode,
    PgTriageReport, PG_TRIAGE_SCHEMA_VERSION,
};
use predicates::prelude::*;
use std::collections::BTreeMap;
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
    let mut content = String::new();

    for entry in 0..num_entries {
        let process_id = 10_000 + entry;
        content.push_str(&format!(
            "2024-01-15 10:00:{:02}.000 UTC [{}] testuser@testdb psql: LOG: statement: SELECT * FROM users WHERE id = {};\n",
            entry % 60,
            process_id,
            entry + 1
        ));
        content.push_str(&format!(
            "2024-01-15 10:00:{:02}.010 UTC [{}] testuser@testdb psql: LOG: duration: 15.234 ms\n",
            entry % 60,
            process_id
        ));
    }

    content
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
    "query_family:qf_c05e64f15dea15ce"
}

fn repo_fixture(path: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(path)
}

fn golden_fixture(path: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(path)
}

fn write_inspect_report(dir: &Path, operating_mode: OperatingMode) -> std::path::PathBuf {
    let report_path = dir.join("inspect.json");
    let report = PgTriageReport {
        schema_version: PG_TRIAGE_SCHEMA_VERSION,
        workflow: ActionKind::Inspect,
        operating_mode,
        limitations: Vec::new(),
        verdict: None,
        verdict_reasons: Vec::new(),
        allowed_actions: None,
        blocked_actions: None,
        analysis_window: None,
        source_summary: None,
        next_actions: Vec::new(),
        report_id: None,
        parent_report_id: None,
        selected_action_id: None,
        created_at: None,
        payload: InspectReportPayload {
            database_inspect: DatabaseInspect {
                mode_candidate: operating_mode,
                checks: BTreeMap::new(),
            },
            agent_inspect: AgentInspect {
                active_harness: None,
                codex: passing_agent_target(),
                claude: passing_agent_target(),
                gemini: passing_agent_target(),
            },
            required_checks: Vec::new(),
            failed_checks: Vec::new(),
        },
    };
    fs::write(&report_path, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    report_path
}

fn passing_agent_target() -> AgentTargetInspect {
    AgentTargetInspect {
        status: CheckStatus::Passed,
        installed: true,
        install_location: "ok".to_string(),
    }
}

fn with_log_backed_inspect<'a>(cmd: &'a mut Command, dir: &Path) -> &'a mut Command {
    let report_path = write_inspect_report(dir, OperatingMode::LogBackedOnly);
    let workspace = report_path.parent().unwrap().to_path_buf();
    cmd.env("PG_LOGSTATS_WORKSPACE", workspace)
}

fn with_log_backed_and_live_inspect<'a>(cmd: &'a mut Command, dir: &Path) -> &'a mut Command {
    let report_path = write_inspect_report(dir, OperatingMode::LogBackedAndLive);
    let workspace = report_path.parent().unwrap().to_path_buf();
    cmd.env("PG_LOGSTATS_WORKSPACE", workspace)
}

fn normalize_findings_json(mut value: serde_json::Value) -> serde_json::Value {
    if let Some(obj) = value.as_object_mut() {
        obj.remove("next_actions");
        obj.remove("report_id");
        obj.remove("parent_report_id");
        obj.remove("selected_action_id");
        obj.remove("created_at");
    }
    value
}

fn persisted_report_paths(workspace: &Path) -> Vec<std::path::PathBuf> {
    let reports_dir = workspace.join("reports");
    if !reports_dir.exists() {
        return Vec::new();
    }

    let mut paths: Vec<_> = fs::read_dir(&reports_dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    paths.sort();
    paths
}

fn only_persisted_report_path(workspace: &Path) -> std::path::PathBuf {
    let paths = persisted_report_paths(workspace);
    assert_eq!(paths.len(), 1, "expected exactly one persisted report");
    paths.into_iter().next().unwrap()
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
        .stdout(predicate::str::contains("--input-format"))
        .stdout(predicate::str::contains("top"))
        .stdout(predicate::str::contains("inspect"))
        .stdout(predicate::str::contains("slow-queries"))
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
    with_log_backed_inspect(&mut cmd, temp_dir.path());
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
    with_log_backed_inspect(&mut cmd, temp_dir.path());
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
fn test_inspect_uses_log_input_without_database_access() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "test.log", sample_log_content());
    let workspace = temp_dir.path().join("workspace");
    fs::create_dir_all(&workspace).unwrap();
    let inspect_report = workspace.join("inspect.json");
    let sessions_dir = workspace.join("sessions");

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    let output = cmd
        .env("PG_LOGSTATS_WORKSPACE", &workspace)
        .arg("--output-format")
        .arg("json")
        .arg("--quiet")
        .arg("inspect")
        .arg(log_file.to_str().unwrap())
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(json["workflow"], "inspect");
    assert_eq!(json["operating_mode"], "log_backed_only");
    assert_eq!(
        json["payload"]["database_inspect"]["checks"]["pg_stat_activity_probe"]["status"],
        "skipped"
    );
    assert_eq!(
        json["payload"]["database_inspect"]["checks"]["statement_evidence"]["status"],
        "passed"
    );
    assert!(inspect_report.exists());
    assert!(!sessions_dir.exists());
}

#[test]
fn test_inspect_defaults_to_json_output() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "test.log", sample_log_content());
    let workspace = temp_dir.path().join("workspace");
    fs::create_dir_all(&workspace).unwrap();

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    let output = cmd
        .env("PG_LOGSTATS_WORKSPACE", &workspace)
        .arg("--quiet")
        .arg("inspect")
        .arg(log_file.to_str().unwrap())
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(json["workflow"], "inspect");
    assert_eq!(json["operating_mode"], "log_backed_only");
}

#[test]
fn test_inspect_without_logs_or_database_is_unready() {
    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    let output = cmd
        .arg("--output-format")
        .arg("json")
        .arg("--quiet")
        .arg("inspect")
        .output()
        .unwrap();

    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();

    assert_eq!(json["operating_mode"], "unready");
    assert_eq!(
        json["payload"]["database_inspect"]["checks"]["log_source_reachable"]["status"],
        "skipped"
    );
    assert!(json["limitations"]
        .as_array()
        .unwrap()
        .iter()
        .any(|value| value == "database_connection_not_configured"));
}

#[test]
fn test_top_query_families_can_follow_persisted_inspect_output() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "test.log", sample_log_content());
    let workspace = temp_dir.path().join("workspace");
    fs::create_dir_all(&workspace).unwrap();

    Command::cargo_bin("pg-logstats")
        .unwrap()
        .env("PG_LOGSTATS_WORKSPACE", &workspace)
        .arg("--output-format")
        .arg("json")
        .arg("--quiet")
        .arg("inspect")
        .arg(log_file.to_str().unwrap())
        .assert()
        .success();

    Command::cargo_bin("pg-logstats")
        .unwrap()
        .env("PG_LOGSTATS_WORKSPACE", &workspace)
        .arg("--output-format")
        .arg("json")
        .arg("--quiet")
        .arg("top")
        .arg("query-families")
        .arg(log_file.to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "\"workflow\": \"top_query_families\"",
        ));

    assert_eq!(persisted_report_paths(&workspace).len(), 1);
    assert!(!workspace.join("sessions").exists());
}

#[test]
fn test_repeated_root_workflows_persist_distinct_reports() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "test.log", sample_log_content());
    let workspace = temp_dir.path().join("workspace");
    fs::create_dir_all(&workspace).unwrap();

    Command::cargo_bin("pg-logstats")
        .unwrap()
        .env("PG_LOGSTATS_WORKSPACE", &workspace)
        .arg("--quiet")
        .arg("inspect")
        .arg(log_file.to_str().unwrap())
        .assert()
        .success();

    for _ in 0..2 {
        Command::cargo_bin("pg-logstats")
            .unwrap()
            .env("PG_LOGSTATS_WORKSPACE", &workspace)
            .arg("--quiet")
            .arg("top")
            .arg("query-families")
            .arg(log_file.to_str().unwrap())
            .assert()
            .success();
    }

    let reports = persisted_report_paths(&workspace);
    assert_eq!(reports.len(), 2);
    assert_ne!(reports[0], reports[1]);
}

#[test]
fn test_log_directory_processing() {
    let temp_dir = TempDir::new().unwrap();
    create_test_log_file(temp_dir.path(), "postgres.log", sample_log_content());
    create_test_log_file(temp_dir.path(), "queries.log", sample_log_content());

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    with_log_backed_inspect(&mut cmd, temp_dir.path());
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
    with_log_backed_inspect(&mut cmd, temp_dir.path());
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
    with_log_backed_inspect(&mut cmd, temp_dir.path());
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
    with_log_backed_inspect(&mut cmd, temp_dir.path());
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
    with_log_backed_inspect(&mut cmd, temp_dir.path());
    cmd.arg("--output-format")
        .arg("text")
        .arg("--quiet")
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
    with_log_backed_inspect(&mut cmd, temp_dir.path());
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
fn test_slow_queries_without_subcommand_requires_inspect_first() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.env("PG_LOGSTATS_WORKSPACE", temp_dir.path())
        .arg("slow-queries")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Run `pg-logstats inspect` first"));
}

#[test]
fn test_slow_queries_without_subcommand_requires_log_backed_readiness() {
    let temp_dir = TempDir::new().unwrap();
    write_inspect_report(temp_dir.path(), OperatingMode::Unready);

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.env("PG_LOGSTATS_WORKSPACE", temp_dir.path())
        .arg("slow-queries")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "This command requires log-backed capability, but inspect reported unready.",
        ));
}

#[test]
fn test_slow_queries_without_subcommand_points_to_canonical_first_steps_after_inspect() {
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    with_log_backed_inspect(&mut cmd, temp_dir.path());
    cmd.arg("slow-queries")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "`pg-logstats slow-queries` is not the first slow-query triage step.",
        ))
        .stderr(predicate::str::contains(
            "pg-logstats inspect --output-format json /path/to/postgresql.log",
        ))
        .stderr(predicate::str::contains(
            "pg-logstats top query-families --output-format json /path/to/postgresql.log",
        ))
        .stderr(predicate::str::contains(
            "pg-logstats slow-queries diff --baseline ... --target ...",
        ))
        .stderr(
            predicate::str::contains("Usage: pg-logstats slow-queries [OPTIONS] <COMMAND>").not(),
        );
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
    with_log_backed_inspect(&mut cmd, temp_dir.path());
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
fn test_run_action_sql_missing_db() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "test.log", sample_log_content());
    let findings_file = temp_dir.path().join("findings.json");

    let mut top_cmd = Command::cargo_bin("pg-logstats").unwrap();
    with_log_backed_and_live_inspect(&mut top_cmd, temp_dir.path());
    top_cmd
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

    let workspace_path = temp_dir.path();
    let report_path = only_persisted_report_path(workspace_path);
    let content = std::fs::read_to_string(&report_path).unwrap();
    let mut report: PgTriageReport<FindingsPayload> = serde_json::from_str(&content).unwrap();
    report.verdict = Some(pg_logstats::Verdict::Clear);
    pg_logstats::populate_next_actions(&mut report, &pg_logstats::AppConfig::default());
    std::fs::write(&report_path, serde_json::to_string_pretty(&report).unwrap()).unwrap();

    let mut run_act_cmd = Command::cargo_bin("pg-logstats").unwrap();
    with_log_backed_and_live_inspect(&mut run_act_cmd, temp_dir.path());
    run_act_cmd
        .arg("--triage-report")
        .arg(report.report_id.as_deref().unwrap())
        .arg("--action-id")
        .arg(format!(
            "query_family.pg_stat_activity.by_dimensions:{}",
            finding_id_for_users_select()
        ))
        .arg("run-sql")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Database connection not configured",
        ));
}

#[test]
fn test_run_action_unknown_or_blocked() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "test.log", sample_log_content());
    let findings_file = temp_dir.path().join("findings.json");

    let mut top_cmd = Command::cargo_bin("pg-logstats").unwrap();
    with_log_backed_and_live_inspect(&mut top_cmd, temp_dir.path());
    top_cmd
        .arg("--output-format")
        .arg("json")
        .arg("--outfile")
        .arg(findings_file.to_str().unwrap())
        .arg("--quiet")
        .arg("top")
        .arg("query-families")
        .arg(log_file.to_str().unwrap())
        .assert()
        .success();

    let mut run_act_cmd = Command::cargo_bin("pg-logstats").unwrap();
    with_log_backed_and_live_inspect(&mut run_act_cmd, temp_dir.path());
    run_act_cmd
        .arg("--triage-report")
        .arg(
            only_persisted_report_path(temp_dir.path())
                .to_str()
                .unwrap(),
        )
        .arg("--action-id")
        .arg("nonexistent_action_id")
        .arg("run-sql")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "not found in parent report next_actions",
        ));
}

#[test]
fn test_empty_log_file() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "empty.log", "");

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    with_log_backed_inspect(&mut cmd, temp_dir.path());
    cmd.arg(log_file.to_str().unwrap())
        .arg("--quiet")
        .arg("top")
        .arg("query-families")
        .assert()
        .failure(); // Should exit with error code for no entries
}

#[test]
fn test_nonexistent_log_file() {
    let temp_dir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    with_log_backed_inspect(&mut cmd, temp_dir.path());
    cmd.arg("--quiet")
        .arg("top")
        .arg("query-families")
        .arg("nonexistent.log")
        .assert()
        .failure(); // Should exit with error code
}

#[test]
fn test_nonexistent_log_directory() {
    let temp_dir = TempDir::new().unwrap();
    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    with_log_backed_inspect(&mut cmd, temp_dir.path());
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
    with_log_backed_inspect(&mut cmd, temp_dir.path());
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
    with_log_backed_inspect(&mut cmd, temp_dir.path());
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
    with_log_backed_inspect(&mut cmd, temp_dir.path());
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
    with_log_backed_inspect(&mut cmd, temp_dir.path());
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
    let fixture = repo_fixture("tests/fixtures/cli/sample_stderr.log");
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    with_log_backed_inspect(&mut cmd, temp_dir.path());
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
    let fixture = repo_fixture("tests/fixtures/cli/sample_stderr.log");
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    with_log_backed_inspect(&mut cmd, temp_dir.path());
    cmd.arg("--output-format")
        .arg("text")
        .arg("top")
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
fn test_checked_in_aws_rds_fixture_auto_detect_smoke() {
    let fixture = repo_fixture("tests/fixtures/cli/aws_rds.log");
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    with_log_backed_inspect(&mut cmd, temp_dir.path());
    cmd.arg("--output-format")
        .arg("text")
        .arg("top")
        .arg("query-families")
        .arg("--quiet")
        .arg(fixture.to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("Findings"))
        .stdout(predicate::str::contains(
            "SELECT COUNT(*) FROM orders WHERE created_at >= ?",
        ))
        .stdout(predicate::str::contains("120.000 ms total runtime"))
        .stdout(predicate::str::contains("SELECT * FROM users WHERE id = ?"));
}

#[test]
fn test_checked_in_aws_rds_fixture_explicit_input_format_marks_evidence() {
    let fixture = repo_fixture("tests/fixtures/cli/aws_rds.log");
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    with_log_backed_inspect(&mut cmd, temp_dir.path());
    cmd.arg("top")
        .arg("query-families")
        .arg("--quiet")
        .arg("--output-format")
        .arg("json")
        .arg("--input-format")
        .arg("rds")
        .arg(fixture.to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("\"entries_scanned\": 5"))
        .stdout(predicate::str::contains("\"source_kind\": \"AwsRds\""))
        .stdout(predicate::str::contains("\"execution_count\": 2"))
        .stdout(predicate::str::contains("\"application_name\": null"))
        .stdout(predicate::str::contains("\"missing_attribution\": ["))
        .stdout(predicate::str::contains("\"application_name\""));
}

#[test]
fn test_startup_fails_without_inspect_output() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "test.log", sample_log_content());
    let workspace = temp_dir.path().join("workspace");
    fs::create_dir_all(&workspace).unwrap();

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.env("PG_LOGSTATS_WORKSPACE", workspace)
        .arg("--quiet")
        .arg("top")
        .arg("query-families")
        .arg(log_file.to_str().unwrap())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Run `pg-logstats inspect` first"));
}

#[test]
fn test_errors_require_inspect_output() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "errors.log", sample_log_content());
    let workspace = temp_dir.path().join("workspace");
    fs::create_dir_all(&workspace).unwrap();

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.env("PG_LOGSTATS_WORKSPACE", workspace)
        .arg("--quiet")
        .arg("errors")
        .arg(log_file.to_str().unwrap())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Run `pg-logstats inspect` first"));
}

#[test]
fn test_temp_files_require_inspect_output() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "temp.log", sample_log_content());
    let workspace = temp_dir.path().join("workspace");
    fs::create_dir_all(&workspace).unwrap();

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.env("PG_LOGSTATS_WORKSPACE", workspace)
        .arg("--quiet")
        .arg("temp-files")
        .arg(log_file.to_str().unwrap())
        .assert()
        .failure()
        .stderr(predicate::str::contains("Run `pg-logstats inspect` first"));
}

#[test]
fn test_running_queries_require_inspect_output() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path().join("workspace");
    fs::create_dir_all(&workspace).unwrap();

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.env("PG_LOGSTATS_WORKSPACE", workspace)
        .arg("running-queries")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Run `pg-logstats inspect` first"));
}

#[test]
fn test_run_sql_require_inspect_output() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path().join("workspace");
    fs::create_dir_all(&workspace).unwrap();

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.env("PG_LOGSTATS_WORKSPACE", workspace)
        .arg("run-sql")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Run `pg-logstats inspect` first"));
}

#[test]
fn test_top_query_families_require_log_backed_readiness() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "test.log", sample_log_content());
    write_inspect_report(temp_dir.path(), OperatingMode::Unready);

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.env("PG_LOGSTATS_WORKSPACE", temp_dir.path())
        .arg("--quiet")
        .arg("top")
        .arg("query-families")
        .arg(log_file.to_str().unwrap())
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "This command requires log-backed capability, but inspect reported unready.",
        ));
}

#[test]
fn test_running_queries_require_ready_mode() {
    let temp_dir = TempDir::new().unwrap();
    write_inspect_report(temp_dir.path(), OperatingMode::Unready);

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.env("PG_LOGSTATS_WORKSPACE", temp_dir.path())
        .arg("running-queries")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "This command cannot run when inspect reported unready.",
        ));
}

#[test]
fn test_run_sql_require_ready_mode() {
    let temp_dir = TempDir::new().unwrap();
    write_inspect_report(temp_dir.path(), OperatingMode::Unready);

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.env("PG_LOGSTATS_WORKSPACE", temp_dir.path())
        .arg("run-sql")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "This command cannot run when inspect reported unready.",
        ));
}

#[test]
fn test_run_sql_requires_triage_report_and_action_id_after_inspect() {
    let temp_dir = TempDir::new().unwrap();
    write_inspect_report(temp_dir.path(), OperatingMode::LogBackedAndLive);

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.env("PG_LOGSTATS_WORKSPACE", temp_dir.path())
        .arg("run-sql")
        .assert()
        .failure()
        .stderr(predicate::str::contains("run-sql requires --triage-report"));
}

#[test]
fn test_errors_require_log_backed_readiness() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "errors.log", sample_log_content());
    write_inspect_report(temp_dir.path(), OperatingMode::Unready);

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.env("PG_LOGSTATS_WORKSPACE", temp_dir.path())
        .arg("--quiet")
        .arg("errors")
        .arg(log_file.to_str().unwrap())
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "This command requires log-backed capability, but inspect reported unready.",
        ));
}

#[test]
fn test_temp_files_require_log_backed_readiness() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "temp.log", sample_log_content());
    write_inspect_report(temp_dir.path(), OperatingMode::Unready);

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.env("PG_LOGSTATS_WORKSPACE", temp_dir.path())
        .arg("--quiet")
        .arg("temp-files")
        .arg(log_file.to_str().unwrap())
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "This command requires log-backed capability, but inspect reported unready.",
        ));
}

#[test]
fn test_agent_install_does_not_require_inspect_output() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path().join("workspace");
    fs::create_dir_all(&workspace).unwrap();

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.env("PG_LOGSTATS_WORKSPACE", workspace)
        .env("HOME", temp_dir.path())
        .arg("--output-format")
        .arg("json")
        .arg("agent")
        .arg("install")
        .arg("--harness")
        .arg("codex")
        .arg("--status")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"workflow\": \"agent_install\""));
}

#[test]
fn test_agent_install_unsupported_harness_has_clean_error() {
    let temp_dir = TempDir::new().unwrap();
    let workspace = temp_dir.path().join("workspace");
    fs::create_dir_all(&workspace).unwrap();

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    cmd.env("PG_LOGSTATS_WORKSPACE", workspace)
        .env("HOME", temp_dir.path())
        .arg("agent")
        .arg("install")
        .arg("--harness")
        .arg("bogus")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unsupported harness: bogus"));
}

#[test]
fn test_cloudwatch_rds_input_uses_fixture_events() {
    let temp_dir = TempDir::new().unwrap();
    let cloudwatch_fixture = create_test_log_file(
        temp_dir.path(),
        "cloudwatch-response.json",
        r#"
{
  "events": [
    {
      "timestamp": 1570000000000,
      "message": "2019-09-24 17:19:25 UTC:172.31.10.173(53224):app@appdb:[12829]:LOG:  statement: SELECT * FROM users WHERE id = 1;"
    },
    {
      "timestamp": 1570000001000,
      "message": "2019-09-24 17:19:25 UTC:172.31.10.173(53224):app@appdb:[12829]:LOG:  duration: 44.000 ms"
    }
  ]
}
"#,
    );

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    with_log_backed_inspect(&mut cmd, temp_dir.path())
        .env("PG_LOGSTATS_CLOUDWATCH_FIXTURE", cloudwatch_fixture)
        .arg("top")
        .arg("query-families")
        .arg("--quiet")
        .arg("--output-format")
        .arg("json")
        .arg("--rds-instance")
        .arg("app-prod")
        .arg("--since")
        .arg("2h")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"source_kind\": \"AwsRds\""))
        .stdout(predicate::str::contains("\"entries_scanned\": 2"))
        .stdout(predicate::str::contains("SELECT * FROM users WHERE id = ?"))
        .stdout(predicate::str::contains("\"total_duration_ms\": 44.0"));
}

#[test]
fn test_cloudwatch_input_rejects_local_files() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "test.log", sample_log_content());

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    with_log_backed_inspect(&mut cmd, temp_dir.path());
    cmd.arg("top")
        .arg("query-families")
        .arg("--quiet")
        .arg("--cloudwatch-log-group")
        .arg("/aws/rds/instance/app-prod/postgresql")
        .arg(log_file.to_str().unwrap())
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "CloudWatch input cannot be combined with local log files",
        ));
}

#[test]
fn test_checked_in_slow_query_diff_fixture_smoke() {
    let baseline = repo_fixture("tests/fixtures/cli/diff_baseline.log");
    let target = repo_fixture("tests/fixtures/cli/diff_target.log");
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    with_log_backed_inspect(&mut cmd, temp_dir.path());
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
fn test_checked_in_run_action_happy_path() {
    let fixture = repo_fixture("tests/fixtures/cli/sample_stderr.log");
    let temp_dir = TempDir::new().unwrap();
    let findings_file = temp_dir.path().join("findings.json");

    let mut top_cmd = Command::cargo_bin("pg-logstats").unwrap();
    with_log_backed_and_live_inspect(&mut top_cmd, temp_dir.path());
    top_cmd
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

    let report_path = only_persisted_report_path(temp_dir.path());
    let content = std::fs::read_to_string(&report_path).unwrap();
    let mut report: PgTriageReport<FindingsPayload> = serde_json::from_str(&content).unwrap();
    report.verdict = Some(pg_logstats::Verdict::Clear);
    pg_logstats::populate_next_actions(&mut report, &pg_logstats::AppConfig::default());
    std::fs::write(&report_path, serde_json::to_string_pretty(&report).unwrap()).unwrap();

    let mut run_act_cmd = Command::cargo_bin("pg-logstats").unwrap();
    with_log_backed_and_live_inspect(&mut run_act_cmd, temp_dir.path());
    run_act_cmd
        .arg("--triage-report")
        .arg(report.report_id.as_deref().unwrap())
        .arg("--action-id")
        .arg(format!(
            "query_family.pg_stat_activity.by_dimensions:query_family:{}",
            "qf_51125b8829ab1fdf"
        ))
        .arg("run-sql")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Database connection not configured",
        ));
}

#[test]
fn test_top_query_families_text_golden() {
    let fixture = repo_fixture("tests/fixtures/cli/sample_stderr.log");
    let expected = fs::read_to_string(golden_fixture("top_query_families_sample.txt")).unwrap();
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    let output = with_log_backed_inspect(&mut cmd, temp_dir.path())
        .arg("--output-format")
        .arg("text")
        .arg("top")
        .arg("query-families")
        .arg("--quiet")
        .arg(fixture.to_str().unwrap())
        .output()
        .unwrap();

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
}

#[test]
fn test_top_query_families_json_golden() {
    let fixture = repo_fixture("tests/fixtures/cli/sample_stderr.log");
    let expected: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(golden_fixture("top_query_families_sample.json")).unwrap(),
    )
    .unwrap();
    let temp_dir = TempDir::new().unwrap();

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    let output = with_log_backed_inspect(&mut cmd, temp_dir.path())
        .arg("top")
        .arg("query-families")
        .arg("--quiet")
        .arg("--output-format")
        .arg("json")
        .arg(fixture.to_str().unwrap())
        .output()
        .unwrap();

    assert!(output.status.success());
    let actual: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be JSON");
    assert_eq!(normalize_findings_json(actual), expected);
}

#[test]
fn test_run_action_from_checked_in_findings_json() {
    let findings = golden_fixture("top_query_families_sample.json");
    let temp_dir = TempDir::new().unwrap();

    let content = fs::read_to_string(&findings).unwrap();
    let mut report: PgTriageReport<FindingsPayload> = serde_json::from_str(&content).unwrap();
    report.operating_mode = OperatingMode::LogBackedAndLive;
    report.verdict = Some(pg_logstats::Verdict::Clear);
    let config = AppConfig::default();
    pg_logstats::populate_next_actions(&mut report, &config);

    let reports_dir = temp_dir.path().join("reports");
    fs::create_dir_all(&reports_dir).unwrap();
    let test_report_path = reports_dir.join("golden-top-query-families.json");
    fs::write(
        &test_report_path,
        serde_json::to_string_pretty(&report).unwrap(),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    with_log_backed_and_live_inspect(&mut cmd, temp_dir.path())
        .arg("--triage-report")
        .arg(test_report_path.to_str().unwrap())
        .arg("--action-id")
        .arg(format!(
            "query_family.pg_stat_activity.by_dimensions:query_family:{}",
            "qf_51125b8829ab1fdf"
        ))
        .arg("run-sql")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Database connection not configured",
        ));
}

#[test]
fn test_run_action_rejects_conflicting_parameter_override() {
    let fixture = repo_fixture("tests/fixtures/cli/sample_stderr.log");
    let temp_dir = TempDir::new().unwrap();
    let findings_file = temp_dir.path().join("findings.json");

    let mut top_cmd = Command::cargo_bin("pg-logstats").unwrap();
    with_log_backed_and_live_inspect(&mut top_cmd, temp_dir.path());
    top_cmd
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

    let report_path = only_persisted_report_path(temp_dir.path());
    let content = std::fs::read_to_string(&report_path).unwrap();
    let mut report: PgTriageReport<FindingsPayload> = serde_json::from_str(&content).unwrap();
    report.verdict = Some(pg_logstats::Verdict::Clear);
    pg_logstats::populate_next_actions(&mut report, &pg_logstats::AppConfig::default());
    std::fs::write(&report_path, serde_json::to_string_pretty(&report).unwrap()).unwrap();

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    with_log_backed_and_live_inspect(&mut cmd, temp_dir.path())
        .arg("--triage-report")
        .arg(report.report_id.as_deref().unwrap())
        .arg("--action-id")
        .arg("query_family.pg_stat_activity.by_dimensions:query_family:qf_51125b8829ab1fdf")
        .arg("run-sql")
        .arg("--parameter")
        .arg("database=otherdb")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Parameter 'database' conflicts with the selected action context",
        ));
}

#[test]
fn test_large_file_processing() {
    let temp_dir = TempDir::new().unwrap();
    let large_content = large_log_content(1000); // 1000 query executions
    let log_file = create_test_log_file(temp_dir.path(), "large.log", &large_content);

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    with_log_backed_inspect(&mut cmd, temp_dir.path());
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
    with_log_backed_inspect(&mut cmd, temp_dir.path());
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
    with_log_backed_inspect(&mut cmd, temp_dir.path());
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
    with_log_backed_inspect(&mut cmd, temp_dir.path())
        .env("RUST_LOG", "debug")
        .arg("--quiet")
        .arg("top")
        .arg("query-families")
        .arg(log_file.to_str().unwrap())
        .assert()
        .success()
        .stderr(predicate::str::contains("DEBUG"))
        .stderr(predicate::str::contains("Initializing text log parser"));
}

#[test]
fn test_json_output_structure() {
    let temp_dir = TempDir::new().unwrap();
    let log_file = create_test_log_file(temp_dir.path(), "test.log", sample_log_content());

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    let output = with_log_backed_inspect(&mut cmd, temp_dir.path())
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

    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["workflow"], "top_query_families");
    assert_eq!(json["operating_mode"], "log_backed_only");
    assert!(json["analysis_window"].is_object());
    assert!(json["source_summary"].is_object());
    assert!(json["payload"]["findings"].is_array());
    assert!(json["payload"]["findings"][0]["kind"].is_string());
}

#[test]
fn test_performance_with_sample_size() {
    let temp_dir = TempDir::new().unwrap();
    let large_content = large_log_content(10000); // 10,000 log entries
    let log_file = create_test_log_file(temp_dir.path(), "huge.log", &large_content);

    let start = std::time::Instant::now();

    let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
    with_log_backed_inspect(&mut cmd, temp_dir.path())
        .arg("--output-format")
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
        .stdout(predicate::str::contains("\"execution_count\": 50"));

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
        with_log_backed_inspect(&mut cmd, temp_dir.path())
            .arg("--quiet")
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
        with_log_backed_inspect(&mut cmd, temp_dir.path())
            .arg("--output-format")
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

    #[test]
    fn test_running_queries_without_db() {
        let temp_dir = TempDir::new().unwrap();
        let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
        with_log_backed_inspect(&mut cmd, temp_dir.path());
        cmd.arg("running-queries")
            .assert()
            .failure()
            .stderr(predicate::str::contains(
                "Database connection not configured",
            ));
    }

    #[test]
    fn test_errors_subcommand() {
        let temp_dir = TempDir::new().unwrap();
        let log_content = r#"2024-01-15 10:00:06.901 UTC [1237] testuser@testdb psql: ERROR:  42P01: relation "nonexistent_table" does not exist
2024-01-15 10:00:07.123 UTC [1237] testuser@testdb psql: FATAL:  3D000: database "other" does not exist"#;
        let log_file = create_test_log_file(temp_dir.path(), "errors.log", log_content);

        let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
        let output = with_log_backed_inspect(&mut cmd, temp_dir.path())
            .arg("--output-format")
            .arg("json")
            .arg("--quiet")
            .arg("errors")
            .arg(log_file.to_str().unwrap())
            .output()
            .unwrap();

        assert!(output.status.success());
        let json_str = String::from_utf8(output.stdout).unwrap();
        let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(json["schema_version"], 1);
        assert_eq!(json["workflow"], "errors");
        assert_eq!(json["payload"]["findings"][0]["kind"], "error_class");
        assert_eq!(
            json["payload"]["findings"][0]["error_class"]["sqlstate"],
            "3D000"
        );
        assert_eq!(
            json["payload"]["findings"][1]["error_class"]["sqlstate"],
            "42P01"
        );
    }

    #[test]
    fn test_temp_files_subcommand() {
        let temp_dir = TempDir::new().unwrap();
        let log_content = r#"2024-01-15 10:00:00.000 UTC [1234] testuser@testdb psql: LOG:  statement: SELECT * FROM giant_table ORDER BY name;
2024-01-15 10:00:01.000 UTC [1234] testuser@testdb psql: LOG:  temporary file: path "base/pgsql_tmp/1234.0", size 5000000 bytes"#;
        let log_file = create_test_log_file(temp_dir.path(), "temp_files.log", log_content);

        let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
        let output = with_log_backed_inspect(&mut cmd, temp_dir.path())
            .arg("--output-format")
            .arg("json")
            .arg("--quiet")
            .arg("temp-files")
            .arg(log_file.to_str().unwrap())
            .output()
            .unwrap();

        assert!(output.status.success());
        let json_str = String::from_utf8(output.stdout).unwrap();
        let json: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(json["schema_version"], 1);
        assert_eq!(json["workflow"], "temp_files");
        assert_eq!(json["payload"]["findings"][0]["kind"], "temp_file");
        assert_eq!(
            json["payload"]["findings"][0]["temp_file"]["largest_observed_bytes"],
            5000000
        );
    }

    #[test]
    fn test_agent_install_subcommand() {
        let temp_dir = TempDir::new().unwrap();
        let mut cmd = Command::cargo_bin("pg-logstats").unwrap();
        with_log_backed_inspect(&mut cmd, temp_dir.path())
            .arg("--output-format")
            .arg("json")
            .arg("agent")
            .arg("install")
            .arg("--harness")
            .arg("claude")
            .arg("--dry-run")
            .assert()
            .success();

        let mut cmd2 = Command::cargo_bin("pg-logstats").unwrap();
        with_log_backed_inspect(&mut cmd2, temp_dir.path())
            .arg("--output-format")
            .arg("json")
            .arg("agent")
            .arg("install")
            .arg("--harness")
            .arg("claude")
            .arg("--status")
            .assert()
            .success();
    }
}
