---
title: pg-logstats PostgreSQL Triage Gateway
description: Install pg-logstats, inspect PostgreSQL readiness, and run agent-first triage workflows for slow queries, errors, temp files, and RDS logs.
schema:
  "@context": "https://schema.org"
  "@type": "SoftwareApplication"
  name: "pg-logstats"
  applicationCategory: "DeveloperApplication"
  operatingSystem: "macOS, Linux"
  description: "Agent-first PostgreSQL triage gateway using logs and read-only system views."
  url: "https://pg-logstats.vrajat.com/"
  downloadUrl: "https://crates.io/crates/pg-logstats"
  codeRepository: "https://github.com/vrajat/pg-logstats"
  programmingLanguage: "Rust"
---

# pg-logstats

**pg-logstats is an agent-first PostgreSQL triage gateway.** Instead of granting coding agents arbitrary database access, it gives them packaged, DBA-approved runbooks that combine PostgreSQL logs with read-only system views.

The CLI parses log evidence, checks live readiness when a DSN is configured, gates diagnostic SQL through named actions, and writes an audit trail under a workspace directory. The goal is to let an agent investigate PostgreSQL incidents without becoming a SQL shell.

## What It Does

- Find slow query families from PostgreSQL statement and duration logs.
- Group PostgreSQL errors by SQLSTATE, normalized message, database, user, and application.
- Attribute temporary file spills to nearby statements when `log_temp_files` evidence is available.
- Inspect readiness from logs and PostgreSQL system views before an agent starts triage.
- Run bounded follow-up checks against views such as `pg_stat_activity`, `pg_stat_database`, and `pg_stat_statements` when live access is configured.
- Read Amazon RDS logs from CloudWatch with the optional AWS SDK feature.

## Start Here

1. Install `pg-logstats`.
2. Install the agent guidance with `pg-logstats agent install`.
3. Run `pg-logstats inspect` against a real log source.
4. Run the workflow that matches the incident: `query-families`, `errors`, or `temp-files`.

Minimal setup flow:

```bash
cargo install pg-logstats
pg-logstats agent install --harness codex
pg-logstats agent install --harness codex --status
pg-logstats inspect /path/to/postgresql.log
pg-logstats query-families /path/to/postgresql.log
```

If you need Amazon RDS or CloudWatch support:

```bash
cargo install pg-logstats --features aws-sdk
```

## Why DBAs Adopt pg-logstats

For database administrators, allowing coding agents to investigate issues requires strict boundaries. `pg-logstats` protects your database by constraining what the agent can do:

- **Zero Arbitrary SQL**: Agents are restricted to a pre-approved menu of read-only diagnostic SQL queries. They cannot execute arbitrary query strings or modify data/schemas.
- **Proactive Load Protection**: High-overhead actions (such as `EXPLAIN ANALYZE`) are dynamically blocked if the database health verdict degrades under locks or query saturation.
- **Operator-Facing Handoffs**: The agent resolves first-pass triage and presents recommendations (like index creation or local memory adjustments) rather than raw log dumps.
- **Audit Trail**: The gateway logs agent attempts, parameters, and query results to JSON reports.

## Runbook References

These are the primary documentation guides detailing the packaged PostgreSQL triage runbooks that `pg-logstats` automates for agents:

- [Slow Query Triage](user-guide/top-query-families.md)
  Triaging slow queries by ranking query families and inspecting execution plans.
- [Error Triage](user-guide/errors.md)
  Grouping repeated PostgreSQL errors by SQLSTATE and normalized message.
- [Temporary Files Triage](user-guide/temp-files.md)
  Triaging disk-write pressure from temporary file spills.

## Setup And Safety Controls

Use these guides to configure `pg-logstats` and audit the safety boundaries of the agent gateway:

- [Inspect and Readiness](user-guide/inspect.md)
  Readiness probes, workspace configuration, and operating-mode checks.
- [Investigation Guidance & Policies](runbooks/action-types.md)
  The `next_actions[]` model, safety verdict matrix, and pre-approved built-in SQL actions catalog.
- [RDS and CloudWatch Log Input](user-guide/rds-cloudwatch.md)
  Configuring remote AWS RDS log windows and IAM policy permissions.

## Contributor & Developer References

- [Architecture](development/architecture.md)
- [API Reference](reference/api.md)
- [Development](development/index.md)
