# pg-logstats

**pg-logstats is a PostgreSQL triage gateway that lets AI agents investigate database incidents safely, enforcing strict operational boundaries without direct database access.**

It packages established PostgreSQL triage runbooks into a controlled CLI: inspect the available evidence, rank findings from logs, and execute pre-approved diagnostic SQL through a bounded action model.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

---

## Why Setup pg-logstats for AI Agents?

Giving coding agents direct database access is risky and hard to audit. `pg-logstats` acts as a secure, sandboxed gateway that lets agents perform first-pass database triage under strict controls.

### 1. Safety via Pre-Approved SQL Actions
The agent **cannot** run arbitrary SQL text. All live database interaction goes through structured, parameterized read-only queries built into the gateway (e.g. querying `pg_stat_statements` by exact query ID or filtering `pg_stat_activity` by dimension).

### 2. Database Load Protection (Verdict Matrix)
To prevent diagnostic activity from adding harmful overhead to a stressed database, actions are allowed or blocked dynamically based on the current health **verdict** (e.g., `clear`, `busy`, `saturated`). For example, running `EXPLAIN ANALYZE` (which executes the query) is restricted if the database is under load.

### 3. Packaged Runbooks vs. Agent Judgement
The gateway models specific PostgreSQL operational runbooks (e.g., Slow Query Triage, Temporary Files Triage):
* **`pg-logstats` owns the runbook**: It determines the evidence shape, log-parsing logic, and allowed action graph.
* **The Agent supplies the judgement**: It decides which branch best fits the incident and reviews ranked findings at explicit branch points.

---

## How the Agent Runbook Loop Works

1. **Setup**: The DBA installs `pg-logstats` and the harness-specific agent guidance.
2. **Readiness Probe**: The agent runs `inspect` against the log source to verify that the environment is ready for triage.
3. **Triage**: The agent executes a log-backed runbook (like `query-families` or `temp-files`), which parses logs and ranks findings.
4. **Follow-Up (SQL)**: The agent executes pre-approved live SQL checks (`action_type = "run_sql"`) to correlate logs with live state (e.g. lock contention or query plans).
5. **DBA Recommendation**: The agent concludes by recommending granular, DBA-approved remedial actions (like creating indexes or tuning session `work_mem`) and stopping.

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

For CloudWatch logs, the agent or operator can inspect a bounded RDS log window:
```bash
pg-logstats inspect --rds-instance my-db --since 1h
```

---

## Documentation Index

The documentation is organized specifically for DBAs setting up and auditing the gateway:

### 1. Primary Runbook References
* [Slow Query Triage](docs/user-guide/top-query-families.md) - Triaging slow queries by ranking query families and inspecting execution plans.
* [Temporary Files Triage](docs/user-guide/temp-files.md) - Triaging disk-write pressure from temporary file spills.

### 2. Setup & Safety Controls
* [Inspect and Readiness](docs/user-guide/inspect.md) - Readiness probes, workspace configuration, and operating-mode checks.
* [Investigation Guidance & Policies](docs/user-guide/guidance.md) - The `next_actions[]` model, safety verdict matrix, and pre-approved SQL actions catalog.
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
