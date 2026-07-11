---
title: pg-logstats Setup for PostgreSQL Triage
description: Install pg-logstats, add agent guidance, and verify PostgreSQL log-backed or RDS-backed triage readiness before deeper investigation.
schema:
  "@context": "https://schema.org"
  "@type": "TechArticle"
  headline: "pg-logstats Setup for PostgreSQL Triage"
  description: "Install pg-logstats, add agent guidance, and verify readiness for PostgreSQL log-backed triage."
  url: "https://pg-logstats.vrajat.com/setup/"
---

# Setup

Bring `pg-logstats` into the environment before the agent starts touching a
PostgreSQL incident.

The setup path has three stages:

1. install the CLI
2. install the agent guidance
3. verify that the environment is ready for the workflow you want to run

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

Before the agent starts deeper triage, run `inspect` against the actual evidence
source you expect the agent to use. The result is persisted to the workspace and
required by later workflows.

Local logs:

```bash
pg-logstats inspect /path/to/postgresql.log
pg-logstats query-families /path/to/postgresql.log
```

RDS / CloudWatch:

```bash
pg-logstats inspect \
  --rds-instance my-db \
  --since 1h

pg-logstats query-families \
  --rds-instance my-db \
  --since 1h
```

The most common success path is `log_backed_only`: logs are available and live
database access is not configured. If a DSN is configured and the required
PostgreSQL probes pass, `inspect` reports `log_backed_and_live` and the reports
can include live follow-up actions. If the required evidence is missing, the
honest result is `unready`.

## Choose A First Workflow

- Use `pg-logstats query-families` for slow queries and latency incidents.
- Use `pg-logstats errors` for repeated PostgreSQL errors, failed statements, or connection failures.
- Use `pg-logstats temp-files` for disk pressure caused by temporary file spills.
- Use `pg-logstats running-queries` only when `inspect` reports live database capability.

## Read Next

- [Inspect and Readiness](../user-guide/inspect.md)
- [RDS and CloudWatch](../user-guide/rds-cloudwatch.md)
- [Runbook Model](../runbooks/index.md)
