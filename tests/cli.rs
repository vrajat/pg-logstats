#[cfg(test)]
mod tests {
    use assert_cmd::Command;
    use std::process::Output;

    #[test]
    fn test_arguments() {
        let mut cmd = Command::cargo_bin("pg-loggrep").unwrap();
        let assert = cmd.args(&[
            "-A", "10", "-c", "localhost", "-D", "-E", "-G", "-N", "app1", "-H", "/html",
            "-i", "postgres", "-I", "-j", "4", "-J", "2", "-l", "last_parsed", "-L", "logfile_list",
            "-m", "1000", "-M", "-n", "-o", "outfile", "-O", "outdir", "-p", "prefix", "-P",
            "-q", "-Q", "-r", "remote_host", "-R", "5", "-s", "3", "-S", "-T", "title", "-u",
            "dbuser", "-U", "exclude_user", "-v", "-w", "-W", "-x", "extension", "-X", "-z",
            "zcat", "-Z", "timezone", "--pie-limit", "10", "--exclude-query", "^(VACUUM|COMMIT)",
            "--exclude-file", "exclude_file", "--include-query", "(tbl1|tbl2)",
            "--include-file", "include_file", "--disable-error", "--disable-hourly",
            "--disable-type", "--disable-query", "--disable-session", "--disable-connection",
            "--disable-lock", "--disable-temporary", "--disable-checkpoint", "--disable-autovacuum",
            "--charset", "utf-8", "--csv-separator", ",", "--exclude-time", "2013-04-12 .*",
            "--include-time", "2013-04-12 .*", "--exclude-db", "pg_dump", "--exclude-appname",
            "pg_dump", "--exclude-line", "exclude_line", "--exclude-client", "exclude_client",
            "--anonymize", "--noreport", "--log-duration", "--enable-checksum",
            "--journalctl", "journalctl -u postgresql-9.5", "--pid-dir", "/tmp",
            "--pid-file", "pgbadger.pid", "--rebuild", "--pgbouncer-only", "--start-monday",
            "--iso-week-number", "--normalized-only", "--noexplain", "--command", "command",
            "--no-process-info", "--dump-all-queries", "--keep-comments", "--no-progressbar"
        ]).assert();

        let output: Output = assert.get_output().clone();
        assert!(output.status.success());
    }

    use predicates::prelude::*;

    #[test]
    fn test_format_argument() {
        let formats = vec!["Syslog", "Syslog2", "Stderr", "Jsonlog", "Cvs", "Pgbouncer",
                                      "Logplex", "Rds", "Redshift"];
        for format in formats {
            let mut cmd = Command::cargo_bin("pg-loggrep").unwrap();
            let lowercase_format = format.to_lowercase();
            let assert = cmd.args(&["--format", &lowercase_format]).assert();
            assert.success().stdout(predicate::str::contains(format!("The format is: {}", format)));
        }
    }
}