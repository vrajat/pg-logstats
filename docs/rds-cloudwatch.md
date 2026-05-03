# Amazon RDS CloudWatch Input

`pg-logstats` can read Amazon RDS for PostgreSQL logs from CloudWatch Logs and
emit compact findings for humans, scripts, and LLM agents.

This is the preferred path for remote RDS investigation because it avoids manual
log downloads and keeps each run bounded to an explicit time window.

## Prerequisites

1. Publish RDS PostgreSQL logs to CloudWatch Logs.

   In the RDS console, modify the DB instance and enable PostgreSQL log exports.
   The default log group shape is:

   ```text
   /aws/rds/instance/<db-instance-id>/postgresql
   ```

2. Install and authenticate the AWS CLI.

   `pg-logstats` shells out to:

   ```bash
   aws logs filter-log-events
   ```

   This means normal AWS CLI auth, SSO, region, and profile behavior applies.

3. Use a time-bounded query.

   CloudWatch input defaults to `--since 1h`. Prefer small windows for LLM
   workflows so the CLI can rank evidence before anything reaches the model.

## Basic Usage

Analyze the last two hours for an RDS instance:

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

Pass the CloudWatch log group explicitly when needed:

```bash
pg-logstats top query-families \
  --cloudwatch-log-group /aws/rds/instance/my-db/postgresql \
  --since 2026-05-03T10:00:00Z \
  --until 2026-05-03T11:00:00Z \
  --output-format json
```

Use AWS profile and region flags:

```bash
pg-logstats top query-families \
  --rds-instance my-db \
  --since 30m \
  --aws-profile prod \
  --aws-region us-east-1 \
  --output-format json
```

## Time Windows

Relative `--since` values support:

- `15m`
- `2h`
- `1d`

Absolute `--since` and `--until` values must be RFC3339 timestamps:

```bash
--since 2026-05-03T10:00:00Z --until 2026-05-03T11:00:00Z
```

`--until` defaults to now.

## Filtering And Page Limits

Use a CloudWatch filter pattern when you want AWS to reduce the event stream
before `pg-logstats` parses it:

```bash
pg-logstats top query-families \
  --rds-instance my-db \
  --since 1h \
  --cloudwatch-filter-pattern '"duration:"' \
  --output-format json
```

CloudWatch input reads up to `--cloudwatch-max-pages` pages. The default is
`20`. Increase it only when the time window is too large or CloudWatch returns
many matching events:

```bash
pg-logstats top query-families \
  --rds-instance my-db \
  --since 6h \
  --cloudwatch-max-pages 100
```

## RDS Log Format

RDS PostgreSQL stderr logs commonly use this prefix:

```text
%t:%r:%u@%d:[%p]:
```

Example:

```text
2019-09-24 17:19:25 UTC:172.31.10.173(53224):app@appdb:[12829]:LOG:  statement: SELECT * FROM users WHERE id = 1;
2019-09-24 17:19:25 UTC:172.31.10.173(53224):app@appdb:[12829]:LOG:  duration: 44.000 ms
```

CloudWatch input defaults auto-detected logs to RDS evidence:

```json
{"source_kind":"AwsRds","record_index":0}
```

## LLM Workflow

For an LLM or agent, prefer JSON output and small windows:

```bash
pg-logstats top query-families \
  --rds-instance my-db \
  --since 30m \
  --limit 10 \
  --output-format json
```

Then ask for follow-up SQL from a selected finding:

```bash
pg-logstats suggest-sql \
  --findings-file findings.json \
  --rank 1
```

This keeps raw log volume out of the LLM context while preserving ranked
findings, normalized SQL, duration metrics, and evidence references.

## Local Fallback

If CloudWatch export is not enabled, download or copy RDS logs locally and use
the RDS parser:

```bash
pg-logstats --input-format rds top query-families postgresql.log.2026-05-03-10
```

## Troubleshooting

- `Failed to run AWS CLI`: install AWS CLI or set `PG_LOGSTATS_AWS_CLI` to a
  compatible executable.
- AWS auth errors: run `aws sts get-caller-identity` with the same profile and
  region.
- No findings: confirm the RDS instance exports PostgreSQL logs to CloudWatch
  and widen `--since`.
- Too much output or slow runs: reduce the time window, add
  `--cloudwatch-filter-pattern`, or lower `--cloudwatch-max-pages`.
