---
title: pg-logstats Docs for PostgreSQL Triage
description: Install pg-logstats, inspect PostgreSQL log readiness, and follow agent-safe runbooks for slow queries, temp files, and RDS log analysis.
schema:
  "@context": "https://schema.org"
  "@type": "SoftwareApplication"
  name: "pg-logstats"
  applicationCategory: "DeveloperApplication"
  operatingSystem: "macOS, Linux"
  description: "Agent-safe PostgreSQL log analysis and triage runbooks for DBAs, operators, and coding agents."
  url: "https://pg-logstats.vrajat.com/"
  downloadUrl: "https://crates.io/crates/pg-logstats"
  codeRepository: "https://github.com/vrajat/pg-logstats"
  programmingLanguage: "Rust"
---

# pg-logstats

**pg-logstats turns PostgreSQL logs into bounded triage reports that humans and coding agents can act on.** It ranks slow query families, groups error classes, attributes temporary file spills, and exposes only DBA-approved follow-up actions when live database access is configured.

Instead of granting coding agents arbitrary database access, `pg-logstats` gives them a PostgreSQL-specific runbook interface. The CLI parses log evidence locally, emits structured JSON reports, gates live diagnostic SQL through named actions, and preserves an auditable incident history under a workspace directory.

## What It Does Today

- Find slow query families from PostgreSQL statement and duration logs.
- Group PostgreSQL errors by SQLSTATE, normalized message, database, user, and application.
- Attribute temporary file spills to nearby statements when `log_temp_files` evidence is available.
- Inspect readiness before an agent starts deeper triage.
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

For database administrators, allowing autonomous coding agents to investigate issues requires strict boundaries. `pg-logstats` acts as a secure gateway that protects your database:

- **Zero Arbitrary SQL**: Agents are restricted to a pre-approved menu of read-only diagnostic SQL queries. They cannot execute arbitrary query strings or modify data/schemas.
- **Proactive Load Protection**: High-overhead actions (such as `EXPLAIN ANALYZE`) are dynamically blocked if the database health verdict degrades under locks or query saturation.
- **Structured Incident Handoffs**: The agent resolves first-pass triage and presents you with structured recommendations (like index creation or local memory adjustments) rather than raw log dumps.
- **Full Audit Trail**: The gateway logs all agent attempts, parameters, and query results to immutable JSON reports, providing a complete audit record.

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
