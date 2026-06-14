# pg-logstats

**pg-logstats is a PostgreSQL triage gateway that lets agents investigate database incidents without direct database access.**

It packages established PostgreSQL triage runbooks into a controlled CLI:
inspect the available evidence, rank the most suspicious findings from logs, and
execute approved follow-up SQL through a bounded action model.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Why Set This Up

Use `pg-logstats` when you want an agent in the investigation loop without
turning that agent into a general-purpose database operator.

This is the intended operating model:

- `pg-logstats` packages the runbook
- the agent supplies judgement at the allowed branch points
- `pg-logstats` remains the gateway for diagnostic SQL

If a human wants to work the logs directly, `pgBadger` is the better tool.

## Quick Start

Install the CLI:

```bash
cargo install pg-logstats
```

Install the agent guidance. `codex` is the first-class target harness:

```bash
pg-logstats agent install --harness codex
pg-logstats agent install --harness codex --status
pg-logstats inspect /path/to/postgresql.log
```

If you want to preview the agent guidance install without writing files:

```bash
pg-logstats agent install --harness codex --dry-run
```

## How The Agent Loop Works

1. A human installs `pg-logstats`.
2. A human installs the agent guidance with `pg-logstats agent install`.
3. The agent runs `inspect`.
4. If the environment is ready, the agent runs one bounded triage workflow.
5. The report returns compact findings plus `next_actions[]`.
6. The agent checks `action_type` on each next action.
7. `pg-logstats run-sql` executes only built-in approved actions with `action_type = "run_sql"`.
8. Delegated branches like "configure DSN and rerun inspect" are `prompt_user` actions, not SQL actions.
9. The agent stops or escalates when the workflow says to stop.

This is not a free-form exploration loop.

## Beta Boundary

For beta, the intended success path is `log_backed`.

If the required evidence is missing, `pg-logstats` should report `unready` and
stop. It should not pretend a weak degraded workflow is acceptable.

That means:

- no historical ranking without supported logs
- no temp-file triage without the required temp-file evidence
- no arbitrary SQL execution through the product path
- no promise that an agent can improvise around missing prerequisites

## Supported Log Inputs

The current text parser supports:

- local PostgreSQL stderr logs with a prefix shaped like `%m [%p] %u@%d %a:`
- Amazon RDS PostgreSQL text logs with a prefix shaped like `%t:%r:%u@%d:[%p]:`

Example stderr shape:

```text
2024-01-15 10:00:00.000 UTC [2001] app@appdb api: LOG:  statement: SELECT * FROM users WHERE id = 1;
2024-01-15 10:00:00.020 UTC [2001] app@appdb api: LOG:  duration: 20.000 ms
```

RDS and CloudWatch-based investigation are also supported. For CloudWatch input,
install with the optional AWS SDK feature:

```bash
cargo install pg-logstats --features aws-sdk
```

Then the agent or operator can inspect a bounded RDS log window through:

```bash
pg-logstats inspect \
  --rds-instance my-db \
  --since 1h
```

## What The Human Docs Are For

The docs in `docs/` are for expert humans setting up `pg-logstats` for agents.

They are meant to answer:

- why `pg-logstats` exists
- how to install it
- how to install the agent guidance
- how to verify that agent triage is trustworthy
- which PostgreSQL runbooks the agent is automating
- where runbook ends and agent judgement begins

They are not meant to be a full command-by-command human CLI manual.

## Read Next

- [Docs Index](docs/index.md)
- [Inspect Reference](docs/user-guide/inspect.md)
- [Investigation Guidance](docs/user-guide/guidance.md)
- [RDS and CloudWatch Input](docs/user-guide/rds-cloudwatch.md)
- [Architecture](docs/development/architecture.md)
- [Development](docs/development/index.md)

## Development

Checked-in fixtures for smoke tests live in [tests/fixtures/cli](tests/fixtures/cli/).

For local development:

```bash
make fmt
make check
```

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE).
