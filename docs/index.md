# pg-logstats

**pg-logstats is a PostgreSQL triage gateway that lets agents investigate database incidents through a controlled CLI.**

It packages established PostgreSQL runbooks into compact findings, explicit
next actions, and approved follow-up SQL. The goal is not to replace `pgBadger`
or `psql` for expert humans. The goal is to give agents a safer and more
auditable path through first-pass database triage.

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

## What pg-logstats Automates

`pg-logstats` does not invent new database diagnostics.

It packages known PostgreSQL triage runbooks into a form that an agent can use
safely:

- compact ranked findings instead of raw log dumps
- explicit `next_actions[]` instead of improvised follow-up steps
- delegated `prompt_user` branches when the operator must decide or add capability
- built-in approved SQL actions instead of arbitrary SQL
- explicit stop or escalate behavior when evidence is insufficient

The core workflow boundary is:

- `pg-logstats` owns the runbook
- the agent owns the judgement at the branch points

If you want to investigate PostgreSQL manually, use `pgBadger`, `psql`, and
your normal SRE or DBA workflow.

## Operating Model

- `pg-logstats` is agent-first. AX wins when it conflicts with human CLI UX.
- `pg-logstats` is the gateway. Agents should not need direct database access.
- Beta success is `log_backed`. If the required evidence is missing, the honest
  result is `unready`.
- The docs are for setup, trust, and auditability, not for teaching manual
  command-by-command usage.

## Key References

### Setup And Trust

- [Inspect](user-guide/inspect.md)
  Current readiness checks, persisted inspect output, and operating-mode
  reporting.
- [Investigation Guidance](user-guide/guidance.md)
  The `next_actions[]` model, delegated `prompt_user` branches, action safety classes, and `run-sql` control path.
- [RDS and CloudWatch Input](user-guide/rds-cloudwatch.md)
  How to provide bounded remote PostgreSQL log windows for agent triage.

### Workflow References

These pages are best read as audit and workflow references, not as primary
human CLI tutorials.

- [Top Query Families](user-guide/top-query-families.md)
  Slow-query triage workflow and ranking model.
- [Errors](user-guide/errors.md)
  Grouped PostgreSQL error triage workflow.
- [Temporary Files](user-guide/temp-files.md)
  Temp-file pressure workflow.

### Contributor References

- [Architecture](development/architecture.md)
- [API Reference](reference/api.md)
- [Development](development/index.md)
