# pg-logstats

**pg-logstats is a PostgreSQL log investigation CLI.**

It reads supported PostgreSQL stderr logs, groups related statements into query families, ranks the most useful findings, and emits bounded follow-up actions for live PostgreSQL inspection.

Use it when you want to triage PostgreSQL database issues, analyze query behavior over specific time windows, or generate safe, actionable guidance for resolving database pressure.

## Supported Workflows

- **inspect**: Check the environment to detect database configuration and agent readiness.
- **top query-families**: Rank query families in a log window by total runtime.
- **slow-queries diff**: Compare query family execution patterns between a target log window and a baseline.
- **run-sql**: Safely execute diagnostic SQL actions with audit linkage and safety checks.
- **errors**: Triage, group, and attribute error logs.
- **temp-files**: Detect temporary file write pressure and correlate them to query families.
- **agent install**: Install playbook instructions and commands into AI agent harnesses.

## Log Prefix Requirements

`pg-logstats` auto-detects local stderr and RDS-style logs. The expected shapes are:

### Local/Standard Stderr Log Prefix Shape
```text
2024-01-15 10:00:00.000 UTC [2001] app@appdb api: LOG:  statement: SELECT * FROM users WHERE id = 1;
2024-01-15 10:00:00.020 UTC [2001] app@appdb api: LOG:  duration: 20.000 ms
```
This corresponds to a `log_line_prefix` resembling:
```text
%m [%p] %u@%d %a:
```

### AWS RDS PostgreSQL Log Prefix Shape
```text
%t:%r:%u@%d:[%p]:
```

Use `--input-format rds` to force RDS parsing or filter for AWS RDS format.

## Quick Start

```bash
# Install the CLI tool
cargo install pg-logstats

# Triage the configuration
pg-logstats inspect --dsn "postgresql://user:password@localhost:5432/dbname"

# Analyze top query families in a stderr log
pg-logstats top query-families sample_stderr.log

# Triage errors in the logs
pg-logstats errors sample_stderr.log

# Triage temporary file resource pressure
pg-logstats temp-files sample_stderr.log
```

## Read Next

- [User Guide](user-guide/index.md)
- [Inspect Workflow](user-guide/inspect.md)
- [Top Query Families Guide](user-guide/top-query-families.md)
- [API Reference](reference/api.md)
- [Architecture](development/architecture.md)
