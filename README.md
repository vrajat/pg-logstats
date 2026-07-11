# pg-logstats

**pg-logstats is an agent-first PostgreSQL triage gateway.** Instead of granting coding agents arbitrary database access, it gives them packaged, DBA-approved runbooks that combine PostgreSQL logs with read-only system views.

The CLI parses log evidence, checks live readiness when a DSN is configured, gates diagnostic SQL through named actions, and writes an audit trail under a workspace directory. The goal is to let an agent investigate PostgreSQL incidents without becoming a SQL shell.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

---

## What It Does

- **Find slow query families** from PostgreSQL statement and duration logs.
- **Group PostgreSQL errors** by SQLSTATE, normalized message, database, user, and application.
- **Attribute temporary file spills** to nearby statements when `log_temp_files` evidence is available.
- **Inspect readiness** from logs and PostgreSQL system views before an agent starts triage.
- **Run bounded follow-up checks** against views such as `pg_stat_activity`, `pg_stat_database`, and `pg_stat_statements` when live access is configured.
- **Read Amazon RDS logs from CloudWatch** with the optional AWS SDK feature.

## Why DBAs Adopt pg-logstats

For database administrators, allowing coding agents to investigate database incidents requires strict boundaries. `pg-logstats` protects your database by constraining what the agent can do:

- **Zero Arbitrary SQL**: Agents are restricted to a pre-approved menu of read-only diagnostic SQL queries. They cannot execute arbitrary query strings or modify data/schemas.
- **Proactive Load Protection**: High-overhead actions (such as `EXPLAIN ANALYZE`) are dynamically blocked if the database health verdict degrades under locks or query saturation.
- **Operator-Facing Handoffs**: The agent resolves first-pass triage and presents recommendations (like index creation or local memory adjustments) rather than raw log dumps.
- **Audit Trail**: The gateway logs agent attempts, parameters, and query results to JSON reports.

---

## The Agent Runbook Loop

The gateway enables a structured, three-phase runbook loop for the agent:
1. **Local Log Triage**: The agent parses PostgreSQL logs offline to rank findings and query families.
2. **Bounded Diagnostic Expansion**: The agent chooses pre-approved, parameter-bound database actions (`run_sql` action class) to check active sessions or execution plans.
3. **Escalation & Remediation**: When the runbook is complete, the agent presents recommendations (like B-Tree indexes or local `work_mem` overrides) directly to the DBA.

---

## Quick Start

Install the CLI:
```bash
cargo install pg-logstats
```

Install the agent guidance (supporting Codex, Claude Code, and Gemini):
```bash
pg-logstats agent install --harness codex
pg-logstats agent install --harness codex --status
pg-logstats inspect /path/to/postgresql.log
pg-logstats query-families /path/to/postgresql.log
```

If the investigation requires Amazon RDS or CloudWatch support, compile with the optional AWS SDK feature:
```bash
cargo install pg-logstats --features aws-sdk
```

---

## Supported Log Inputs

The current text parser supports:
* **Local stderr logs** with a prefix shaped like `%m [%p] %u@%d %a:`
* **Amazon RDS text logs** with a prefix shaped like `%t:%r:%u@%d:[%p]:`

For CloudWatch logs, the agent or operator can inspect and analyze a bounded RDS log window:
```bash
pg-logstats inspect --rds-instance my-db --since 1h
pg-logstats query-families --rds-instance my-db --since 1h
```

---

## Documentation Index

The documentation is organized specifically for DBAs setting up and auditing the gateway:

### 1. Primary Runbook References
* [Slow Query Triage](docs/user-guide/top-query-families.md) - Triaging slow queries by ranking query families and inspecting execution plans.
* [Error Triage](docs/user-guide/errors.md) - Grouping repeated PostgreSQL errors by SQLSTATE and normalized message.
* [Temporary Files Triage](docs/user-guide/temp-files.md) - Triaging disk-write pressure from temporary file spills.

### 2. Setup & Safety Controls
* [Inspect and Readiness](docs/user-guide/inspect.md) - Readiness probes, workspace configuration, and operating-mode checks.
* [Investigation Guidance & Policies](docs/runbooks/action-types.md) - The `next_actions[]` model, safety verdict matrix, and pre-approved SQL actions catalog.
* [RDS and CloudWatch Log Input](docs/user-guide/rds-cloudwatch.md) - Configuring remote AWS RDS log windows and IAM policy permissions.

---

## Local Development

Checked-in fixtures for smoke tests live in [tests/fixtures/cli/](tests/fixtures/cli/).

Run formatters and checks:
```bash
make fmt
make check
```

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE).
