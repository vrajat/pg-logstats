# pg-logstats

`pg-logstats` is a PostgreSQL log investigation CLI. It reads supported
PostgreSQL stderr logs, groups related statements into query families, ranks the
most useful findings, and emits bounded follow-up actions for live PostgreSQL
inspection.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Supported Workflows

- `inspect`: inspect the environment to check database configuration and agent setup.
- `top query-families`: rank query families in one log window by total runtime.
- `slow-queries diff`: compare a target log window against a baseline window.
- `run-sql`: execute a built-in diagnostic SQL action safely with session linkage and safety checks.

Supported input is PostgreSQL stderr logs using this prefix shape:

```text
2024-01-15 10:00:00.000 UTC [2001] app@appdb api: LOG:  statement: SELECT * FROM users WHERE id = 1;
2024-01-15 10:00:00.020 UTC [2001] app@appdb api: LOG:  duration: 20.000 ms
```

That corresponds to a PostgreSQL `log_line_prefix` similar to:

```text
%m [%p] %u@%d %a:
```

Amazon RDS for PostgreSQL logs are also supported when they use the RDS prefix
shape documented for pgBadger:

```text
%t:%r:%u@%d:[%p]:
```

`pg-logstats` auto-detects local stderr and RDS-style logs by default. Use
`--input-format rds` when you want JSON evidence to mark the source kind as
`AwsRds` or when you want to reject non-RDS prefixes.

## Quick Start

```bash
cargo install pg-logstats

pg-logstats top query-families tests/fixtures/cli/sample_stderr.log

pg-logstats --input-format rds top query-families tests/fixtures/cli/aws_rds.log

pg-logstats top query-families \
  --rds-instance my-db \
  --since 2h \
  --output-format json

pg-logstats top query-families \
  --output-format json \
  --outfile findings.json \
  tests/fixtures/cli/sample_stderr.log

pg-logstats slow-queries diff \
  --baseline tests/fixtures/cli/diff_baseline.log \
  --target tests/fixtures/cli/diff_target.log

pg-logstats --session-id test_sess --parent-report-id 0001-top_query_families --selected-action-id query_family.pg_stat_activity.by_dimensions:query_family:qf_51125b8829ab1fdf run-sql
```

Global flags such as `--input-format`, `--output-format`, `--outfile`,
`--outdir`, `--workspace`, `--dsn`, and `--quiet` can be placed before or after the
workflow command.

## CloudWatch Logs Input

For Amazon RDS PostgreSQL instances that publish PostgreSQL logs to CloudWatch
Logs, `pg-logstats` can read a bounded time window through the optional AWS SDK
integration.

Build with the optional feature:

```bash
cargo install pg-logstats --features aws-sdk
```

Then run:

```bash
pg-logstats top query-families \
  --rds-instance my-db \
  --since 2h \
  --output-format json
```

`--rds-instance my-db` resolves to:

```text
/aws/rds/instance/my-db/postgresql
```

You can also pass the log group explicitly:

```bash
pg-logstats top query-families \
  --cloudwatch-log-group /aws/rds/instance/my-db/postgresql \
  --since 2026-05-03T10:00:00Z \
  --until 2026-05-03T11:00:00Z
```

CloudWatch input uses the AWS SDK's normal credential and region provider chain.
Use `--aws-profile`, `--aws-region`, `--cloudwatch-filter-pattern`, and
`--cloudwatch-max-pages` to control the request. Relative `--since` values
support `m`, `h`, and `d`.

## Installation

From crates.io:

```bash
cargo install pg-logstats
pg-logstats --version
```

From a local checkout:

```bash
git clone https://github.com/vrajat/pg-logstats.git
cd pg-logstats
cargo install --path .
pg-logstats --version
```

From source without installing:

```bash
cargo run -- top query-families tests/fixtures/cli/sample_stderr.log
```

## Commands

### Inspect

Inspect the environment and determine the supported operating mode:

```bash
pg-logstats inspect --output-format json
```

Use supported log input to determine `log_backed` mode even when no PostgreSQL
connection is configured:

```bash
pg-logstats inspect \
  --output-format json \
  tests/fixtures/cli/sample_stderr.log
```

When live checks are needed, connection discovery precedence is:

1. `--dsn <postgres-url>`
2. `PG_LOGSTATS_DATABASE_URL`
3. `[database].dsn` from the resolved config

The dedicated inspect guide lives in [docs/inspect.md](docs/inspect.md).

### Top Query Families

Rank normalized query families in one log window:

```bash
pg-logstats top query-families tests/fixtures/cli/sample_stderr.log
```

Analyze every `.log` or `.txt` file in a directory:

```bash
pg-logstats top query-families --log-dir tests/fixtures/cli
```

Limit the number of emitted findings:

```bash
pg-logstats top query-families --limit 5 tests/fixtures/cli/sample_stderr.log
```

Write JSON findings for shell or agent workflows:

```bash
pg-logstats top query-families \
  --output-format json \
  --outfile findings.json \
  tests/fixtures/cli/sample_stderr.log
```

### Slow Query Diff

Compare a target log window with a baseline log window:

```bash
pg-logstats slow-queries diff \
  --baseline tests/fixtures/cli/diff_baseline.log \
  --target tests/fixtures/cli/diff_target.log
```

Apply eligibility thresholds:

```bash
pg-logstats slow-queries diff \
  --baseline tests/fixtures/cli/diff_baseline.log \
  --target tests/fixtures/cli/diff_target.log \
  --min-target-count 2 \
  --min-target-total-ms 100 \
  --min-p95-delta-ms 10
```

### Run SQL / Guidance Action

Execute a recommended action using safety checks and session tracking:

```bash
pg-logstats \
  --session-id test_sess \
  --parent-report-id 0001-top_query_families \
  --selected-action-id query_family.pg_stat_activity.by_dimensions:query_family:qf_51125b8829ab1fdf \
  run-sql
```

For SQL-based actions, the command validates the action's safety using the policy matrix (verdict and action class restrictions) and records the execution step under the session reports directory.

## JSON Output

`top query-families` JSON output now uses the V1 `PgTriageReport` shape:

```bash
pg-logstats top query-families \
  --output-format json \
  tests/fixtures/cli/sample_stderr.log | jq '.payload.findings[0]'
```

Useful fields include:

- `schema_version`
- `workflow`
- `operating_mode`
- `analysis_window.since`
- `analysis_window.until`
- `source_summary.kind`
- `source_summary.entries_scanned`
- `payload.findings[].finding_id`
- `payload.findings[].kind`
- `payload.findings[].rank`
- `payload.findings[].title`
- `payload.findings[].reason`
- `payload.findings[].reason_codes`
- `payload.findings[].score`
- `payload.findings[].confidence`
- `payload.findings[].query_family.normalized_sql`
- `payload.findings[].query_family.database`
- `payload.findings[].query_family.user`
- `payload.findings[].query_family.application_name`
- `payload.findings[].query_family.missing_attribution`
- `payload.findings[].metrics.execution_count`
- `payload.findings[].metrics.total_duration_ms`
- `payload.findings[].metrics.max_duration_ms`
- `payload.findings[].next_sql`

For diff findings, each finding also includes `baseline`, `target`, and `delta`
duration summaries.

`top query-families` is a `log_backed` workflow. It requires PostgreSQL
statement and duration evidence in supported logs. On startup, non-`inspect`
commands require a persisted inspect report. By default that report is written
inside the workspace as `inspect.json`. By default the workspace is
`~/.local/share/pg-logstats`, or you can override it with `--workspace` or
`PG_LOGSTATS_WORKSPACE`. The report window is described by
`analysis_window.{since,until}` and `source_summary.*`.

See [docs/top-query-families.md](docs/top-query-families.md) for the bounded
window model, supported formats, missing-attribution behavior, and the logging
references used during design.

## Configuration

Workspace precedence is:

1. `--workspace <dir>`
2. `PG_LOGSTATS_WORKSPACE`
3. `~/.local/share/pg-logstats`

Within the workspace, `pg-logstats` expects:

- `config.toml`
- `inspect.json`
- `results/` for future command output and cached artifacts

Minimal example:

```toml
[database]
dsn = "postgres://user@host:5432/dbname"
connect_timeout_ms = 3000

[running_queries.thresholds]
long_running_query_ms = 120000
waiting_session_count_threshold = 2
idle_in_transaction_count_threshold = 2

[suggest_sql]
max_risk = "bounded"
show_omitted = true
disabled_rules = []
```

Unknown config keys are rejected. The config loader fails fast instead of
silently ignoring unsupported fields.

## Fixture Logs

[tests/fixtures/cli](tests/fixtures/cli/) contains the checked-in fixture logs
used by the commands above.

## Development

```bash
make fmt
make check
```

Run a smoke command during local development:

```bash
cargo run -- top query-families tests/fixtures/cli/sample_stderr.log
```

## Troubleshooting

If no findings are emitted, first check the log prefix. The current parser expects
the supported stderr prefix shown above and statement/duration lines that can be
correlated by process id and order.

Use `--sample-size <N>` with `top query-families` or `slow-queries diff` when you
want a quick pass over the first N lines of each file.

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE).
