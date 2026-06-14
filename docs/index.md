# pg-logstats

**pg-logstats is an agent-first PostgreSQL triage gateway. Instead of granting coding agents arbitrary database access, it provides them with pre-packaged, DBA-approved PostgreSQL runbooks.**

By bundling database triage logic directly into the gateway CLI, `pg-logstats` translates raw log analysis and diagnostic SQL into a sequence of safe, bounded next actions. The agent only supplies the judgement at explicit branch points, while the gateway enforces security, protects database load, and generates an auditable history of the incident.

## Start Here

1. Install `pg-logstats`.
2. Install the agent guidance with `pg-logstats agent install`.
3. Run `pg-logstats inspect`.
4. Confirm the environment is ready before the agent starts deeper triage.

Minimal setup flow:

```bash
cargo install pg-logstats
pg-logstats agent install --harness codex
pg-logstats agent install --harness codex --status
pg-logstats inspect /path/to/postgresql.log
```

If you need Amazon RDS or CloudWatch support:

```bash
cargo install pg-logstats --features aws-sdk
```

## Why DBAs Adopt pg-logstats

For database administrators, allowing autonomous coding agents to investigate issues requires strict boundaries. `pg-logstats` acts as a secure gateway that protects your database:

- **Zero Arbitrary SQL**: Agents are restricted to a pre-approved menu of read-only diagnostic SQL queries. They cannot execute arbitrary query strings or modify data/schemas.
- **Proactive Load Protection**: High-overhead actions (such as `EXPLAIN ANALYZE`) are dynamically blocked if the database health verdict degrades under locks or query saturation.
- **Structured incident Handoffs**: The agent resolves first-pass triage and presents you with structured recommendations (like index creation or local memory adjustments) rather than raw log dumps.
- **Full Audit Trail**: The gateway logs all agent attempts, parameters, and query results to immutable JSON reports, providing a complete audit record.

## Runbook References

These are the primary documentation guides detailing the packaged PostgreSQL triage runbooks that `pg-logstats` automates for agents:

- [Slow Query Triage](user-guide/top-query-families.md)
  Triaging slow queries by ranking query families and inspecting execution plans.
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
