# pg-logstats

`pg-logstats` is a PostgreSQL log investigation CLI. It reads supported
PostgreSQL stderr logs, groups related statements into query families, ranks the
most useful findings, and prints follow-up SQL for live PostgreSQL inspection.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Supported Workflows

- `top query-families`: rank query families in one log window by total runtime.
- `slow-queries diff`: compare a target log window against a baseline window.
- `suggest-sql`: print `pg_stat_statements` and `pg_stat_activity` follow-up SQL
  for a finding from JSON output.

Supported input is PostgreSQL stderr logs using this prefix shape:

```text
2024-01-15 10:00:00.000 UTC [2001] app@appdb api: LOG:  statement: SELECT * FROM users WHERE id = 1;
2024-01-15 10:00:00.020 UTC [2001] app@appdb api: LOG:  duration: 20.000 ms
```

That corresponds to a PostgreSQL `log_line_prefix` similar to:

```text
%m [%p] %u@%d %a:
```

## Quick Start

```bash
cargo install --path .

pg-logstats top query-families tests/fixtures/cli/sample_stderr.log

pg-logstats top query-families \
  --output-format json \
  --outfile findings.json \
  tests/fixtures/cli/sample_stderr.log

pg-logstats slow-queries diff \
  --baseline tests/fixtures/cli/diff_baseline.log \
  --target tests/fixtures/cli/diff_target.log

pg-logstats suggest-sql --findings-file findings.json --rank 1
```

Global flags such as `--output-format`, `--outfile`, `--outdir`, and `--quiet`
can be placed before or after the workflow command.

## Installation

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

### Suggested SQL

Generate follow-up SQL for a finding selected by rank:

```bash
pg-logstats suggest-sql --findings-file findings.json --rank 1
```

Or select by exact finding id:

```bash
pg-logstats suggest-sql \
  --findings-file findings.json \
  --finding-id 'query_family:queryid=|db=appdb|user=app|app=api|sql=SELECT * FROM users WHERE id = ?'
```

## JSON Output

JSON output is centered on findings:

```bash
pg-logstats top query-families \
  --output-format json \
  tests/fixtures/cli/sample_stderr.log | jq '.findings[0]'
```

Useful fields include:

- `schema_version`
- `metadata.analysis_timestamp`
- `metadata.tool_version`
- `metadata.total_log_entries`
- `findings[].finding_id`
- `findings[].kind`
- `findings[].rank`
- `findings[].title`
- `findings[].reason`
- `findings[].reason_codes`
- `findings[].score`
- `findings[].confidence`
- `findings[].query_family.normalized_sql`
- `findings[].query_family.database`
- `findings[].query_family.user`
- `findings[].query_family.application_name`
- `findings[].metrics.execution_count`
- `findings[].metrics.total_duration_ms`
- `findings[].metrics.max_duration_ms`
- `findings[].next_sql`

For diff findings, each finding also includes `baseline`, `target`, and `delta`
duration summaries.

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
