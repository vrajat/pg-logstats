# Setup

Bring `pg-logstats` into the environment before the agent starts touching a
PostgreSQL incident.

The setup path has three stages:

1. install the CLI
2. install the agent guidance
3. verify that the environment is ready for log-backed triage

## Install

Today the simplest install path is:

```bash
cargo install pg-logstats
```

If the investigation path needs Amazon RDS or CloudWatch input:

```bash
cargo install pg-logstats --features aws-sdk
```

Homebrew packaging is an intended distribution path and should live here once it
is published.

## Install Agent Guidance

The agent should not be expected to discover the product contract on its own.
Install the harness-specific guidance first.

For Codex:

```bash
pg-logstats agent install --harness codex
pg-logstats agent install --harness codex --status
```

Use `--dry-run` when you want to preview the expected file writes:

```bash
pg-logstats agent install --harness codex --dry-run
```

## Verify Readiness

Before the agent starts deeper triage, run `inspect` against the actual log
source you expect the agent to use.

Local logs:

```bash
pg-logstats inspect --output-format json /path/to/postgresql.log
```

RDS / CloudWatch:

```bash
pg-logstats inspect \
  --rds-instance my-db \
  --since 1h \
  --output-format json
```

For beta, the intended success path is `log_backed`. If the required evidence
is missing, the honest result is `unready`.

## Read Next

- [Inspect and Readiness](../user-guide/inspect.md)
- [RDS and CloudWatch](../user-guide/rds-cloudwatch.md)
- [Workflow Model](../workflow-model/index.md)
