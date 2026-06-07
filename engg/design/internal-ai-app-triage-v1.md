# Design: Internal AI App Triage V1

- Status: Draft
- Date: 2026-06-05

## Overview

This document defines the first V1 implementation target for `pg-logstats` as a
**first-pass PostgreSQL triage companion** for internal AI-built applications.

The scope is:

- help an agent or developer determine whether `pg-logstats` can provide useful
  triage support in the current environment
- make `pg-logstats` report whether it is operating with full, reduced, or
  insufficient evidence
- surface compact, ranked, PostgreSQL-specific findings when the evidence allows
- suggest safe next investigative steps and tell the caller when to stop and
  escalate

## Goals

- Make `pg-logstats` useful as a first-pass triage tool for internal
  application-driven Postgres issues.
- Define a clear readiness contract between PostgreSQL, `pg-logstats`, and the
  consuming agent.
- Make degraded operation explicit and machine-readable.
- Ship a narrow CLI surface that is sufficient for first-pass triage.
- Make behavior configurable from V1, including local custom `suggest-sql`
  rules.
- Keep the output compact enough for agent turns and auditable by humans.
- Treat every V1 triage workflow as an attributed, well-known PostgreSQL
  diagnostic pattern rather than a novel pg-logstats invention.

## Primary User Story

An internal application starts causing database pain.

The user, which may be a developer or a coding agent, needs to answer:

1. Can `pg-logstats` act as a useful triage companion in this environment?
2. Is `pg-logstats` operating with full or degraded evidence?
3. Which query family, error class, or temp-file pattern is most suspicious?
4. What safe next investigative steps should be taken?
5. When should this investigation stop and escalate?

## Workflow Attribution And Selection

V1 should only ship triage workflows that are already recognizable PostgreSQL
operational workflows. `pg-logstats` is packaging these workflows into compact,
machine-readable reports; it is not inventing new diagnostic theory.

Each V1 workflow must have explicit attribution before implementation. If a
candidate workflow cannot be tied to PostgreSQL documentation, pgBadger
documentation, a credible PostgreSQL operational write-up, or a well-scoped
mailing-list discussion, it should be skipped or deferred until that rationale is
clear.

Attribution should be recorded in the implementation issue, user-facing docs,
and the workflow documentation. Built-in reason codes and suggested SQL rules
should also be traceable to the same rationale.

Initial V1 implementation reference map:

| Command or Rule Family | Why It Is In V1 | Implementation References |
| --- | --- | --- |
| `readiness` | PostgreSQL evidence quality determines whether log-backed or live-only claims are honest. | PostgreSQL logging configuration docs; PostgreSQL `pg_stat_statements` docs; PostgreSQL predefined roles docs; pgBadger PostgreSQL configuration section for required logging and `log_line_prefix` patterns; pgBadger source header comments for supported logging prerequisites. |
| `running-queries` | Active sessions, waits, query text visibility, and aggregate statement stats are standard first-pass live triage inputs. | PostgreSQL `pg_stat_activity` docs for active sessions, application names, waits, backend state, query text, and `query_id`; PostgreSQL `pg_stat_statements` docs for aggregate query stats; PostgreSQL predefined roles docs for cluster-wide stats visibility. |
| `top query-families` | Ranking slow, frequent, and time-consuming query families is a long-standing PostgreSQL log-analysis workflow. | pgBadger feature docs for slowest queries, time-consuming queries, frequent queries, and users/applications involved in top queries; pgBadger source functions `print_time_consuming`, `print_slowest_individual_queries`, and `print_slowest_queries`; PostgreSQL `pg_stat_statements` docs for query-family aggregate fields and `queryid`. |
| `errors` | Grouped PostgreSQL errors and error classes are common incident-triage signals. | pgBadger feature docs for most frequent errors, error events, and error class distribution; pgBadger source functions `print_error_code`, `show_error_as_html`, and `show_pgb_error_as_html`; PostgreSQL error reporting and logging docs. |
| `temp-files` | Temporary file volume is a PostgreSQL-specific pressure signal, often tied to sorts, hashes, and `work_mem`-sensitive plans. | pgBadger feature docs for queries generating the most temporary files, queries generating the largest temporary files, and temporary-file statistics; pgBadger source functions `print_tempfile_report` and `print_temporary_file`; PostgreSQL `log_temp_files` docs; PostgreSQL `pg_stat_statements` docs for `temp_blks_read` and `temp_blks_written`. |
| `suggest-sql`: query-family rules | Follow-up SQL should bridge a query-family finding into standard PostgreSQL stats and activity views. | pgBadger top-query reports as prior art for preserving query text, user, database, application, and queryid in findings; PostgreSQL `pg_stat_statements` docs for exact `queryid` lookups; PostgreSQL `pg_stat_activity` docs for active-session lookups by database, user, application name, pid, and query id. |
| `suggest-sql`: running-query rules | Live follow-up should inspect one backend or its blocking context without broad scans. | PostgreSQL `pg_stat_activity` docs for backend state, wait event fields, and query text; PostgreSQL docs for `pg_blocking_pids`; PostgreSQL predefined roles docs for visibility limits. |
| `suggest-sql`: error-class rules | Error findings can safely lead to current activity checks by the same app, database, or user, but not historical SQLSTATE queries from catalogs. | pgBadger error reports as prior art for grouping error message, SQLSTATE, database, user, application, and sample details; PostgreSQL error reporting docs; PostgreSQL `pg_stat_activity` docs for bounded live activity checks. |
| `suggest-sql`: temp-file rules | Temp-file findings can safely lead to stats-view checks for database temp counters or query-family temp block counters. | pgBadger temporary-file reports as prior art for ranking by count and size; PostgreSQL `log_temp_files` docs; PostgreSQL `pg_stat_database` docs for database temp counters; PostgreSQL `pg_stat_statements` docs for temp block counters. |

The references must be specific enough for an engineer to inspect the prior-art
workflow shape before implementing a command. For example, an engineer
implementing `pg-logstats temp-files` should inspect pgBadger's feature list,
the `print_tempfile_report` source for ranking by temporary-file count and size,
the `print_temporary_file` source for temporary-file time-series summaries, and
PostgreSQL's `log_temp_files` documentation.

Reference URLs:

The pgBadger source anchors below are implementation starting points. If
upstream line numbers move, search the file for the report headings or function
names listed in the reference map.

- pgBadger feature docs: <https://access.crunchydata.com/documentation/pgbadger/latest/>
- pgBadger report examples: <https://pgbadger.darold.net/#reports>
- pgBadger source, logging prerequisites:
  <https://github.com/darold/pgbadger/blob/master/pgbadger#L1>
- pgBadger source, temporary-file reports:
  <https://github.com/darold/pgbadger/blob/master/pgbadger#L1158-L1187>
- pgBadger source, temporary-file activity charts:
  <https://github.com/darold/pgbadger/blob/master/pgbadger#L804-L832>
- pgBadger source, top-query reports:
  <https://github.com/darold/pgbadger/blob/master/pgbadger#L1255-L1272>
- pgBadger source, normalized slow-query reports:
  <https://github.com/darold/pgbadger/blob/master/pgbadger#L1386-L1389>
- pgBadger source, error class distribution:
  <https://github.com/darold/pgbadger/blob/master/pgbadger#L1609-L1623>
- pgBadger source, frequent errors/events:
  <https://github.com/darold/pgbadger/blob/master/pgbadger#L1665-L1671>
- PostgreSQL logging configuration: <https://www.postgresql.org/docs/current/runtime-config-logging.html>
- PostgreSQL `log_temp_files`: <https://www.postgresql.org/docs/current/runtime-config-logging.html#GUC-LOG-TEMP-FILES>
- PostgreSQL monitoring statistics and `pg_stat_activity`: <https://www.postgresql.org/docs/current/monitoring-stats.html#MONITORING-PG-STAT-ACTIVITY-VIEW>
- PostgreSQL `pg_stat_statements`: <https://www.postgresql.org/docs/current/pgstatstatements.html>
- PostgreSQL predefined roles: <https://www.postgresql.org/docs/current/predefined-roles.html>
- PostgreSQL `pg_blocking_pids`: <https://www.postgresql.org/docs/current/functions-info.html>

## Operating Modes

V1 defines three top-level operating modes.

### `log_backed`

Historical log-backed triage is available.

This mode allows:

- `top query-families`
- `errors`
- `temp-files`
- `suggest-sql`

### `live_only`

Historical logs are unavailable or insufficient, but low-impact live-state
inspection is still possible.

This mode allows:

- `readiness`
- `running-queries`
- limited `suggest-sql` for live-state or aggregate follow-up

This mode may still have some non-log history through sources such as
`pg_stat_statements`, but it does not provide a log-backed historical event
stream.

This mode does not allow `pg-logstats` to claim:

- bounded historical ranking from logs
- event-level log evidence handles
- error or temp-file findings that depend on log events

### `unready`

Neither historical log-backed triage nor useful live-state inspection is
available.

This mode allows:

- `readiness`

This mode should produce an explicit explanation of what is missing.

## Readiness Contracts

### Database Readiness

The database environment is `log_backed`-ready when:

- PostgreSQL logs are available from a supported source
- the log format is supported by `pg-logstats`
- statements and durations are emitted with enough detail for correlation
- app attribution is available when possible through `application_name`

For V1, `log_backed` mode should be determined by explicit checks:

- `SHOW log_destination`
- `SHOW log_line_prefix`
- `SHOW log_duration`
- `SHOW log_min_duration_statement`
- `SHOW log_temp_files`
- direct access to a supported log source

V1 `log_backed` mode should require one of these evidence patterns:

1. currently supported stderr-style or RDS-style logs containing statement lines
   and duration lines that can be correlated by session or process identity
2. a future structured format explicitly supported by the implementation

V1 should treat these PostgreSQL logging settings as the minimum documented
targets for useful log-backed triage, but only when the corresponding log
format is implemented:

- `log_destination` includes a currently implemented supported destination such
  as `stderr`
- `log_duration = on` or `log_min_duration_statement >= 0`
- `log_line_prefix` preserves enough session identity to correlate statements
  and durations
- a sampled reachable log source actually contains statement text and duration
  evidence that the parser can correlate

`csvlog` and `jsonlog` remain documented targets, but must not satisfy V1
`log_backed` readiness until the implementation explicitly supports parsing
them.

For `temp-files`, V1 should additionally require:

- `log_temp_files >= 0`

If these checks fail, `top query-families`, `errors`, or `temp-files` must not
pretend full log-backed support exists.

The database environment is `live_only`-ready when:

- `pg_stat_activity` is queryable
- `pg_stat_statements` is installed and active
- `compute_query_id` is `auto` or `on`, or an equivalent query identifier source
  is active
- the caller can read cluster-wide activity and statement statistics

For V1, `live_only` mode should be a single precise contract. The implementation
should check all of the following and bail to `unready` if any required check
fails:

- `SHOW track_activities`
- `SHOW shared_preload_libraries`
- `SHOW compute_query_id`
- `SELECT extname FROM pg_extension WHERE extname = 'pg_stat_statements'`
- `SELECT pg_has_role(current_user, 'pg_read_all_stats', 'member')`
- a probe query against `pg_stat_activity`
- a probe query against `pg_stat_statements`

The required state for `live_only` is:

- `track_activities = on`
- `shared_preload_libraries` includes `pg_stat_statements`
- `compute_query_id IN ('auto', 'on')`, or the implementation has an explicit
  alternative query-id integration
- `pg_stat_statements` extension exists in the target database
- the caller is superuser or has `pg_read_all_stats`

The database environment is `unready` when:

- supported logs are not available
- or the exact `live_only` requirements above are not satisfied
- or the available evidence is too weak to support safe triage claims

### Agent Readiness

The agent environment is ready when:

- `pg-logstats` has installed the harness-specific guidance bundle for the
  selected agent surface
- that installed guidance tells the agent exactly how to run `readiness` first,
  interpret `operating_mode`, and respect `verdict`, `allowed_actions`, and
  `blocked_actions`
- the agent can invoke `pg-logstats` from the local environment and read JSON
  output

V1 should not assume a git repository. Agent integration should therefore be
user-level, not repo-local.

The installation contract should be:

- `pg-logstats agent install --harness codex`
- `pg-logstats agent install --harness claude`
- `pg-logstats agent install --harness gemini`

The installed artifacts should be:

- Codex:
  - install a managed pg-logstats block into the user-scoped `AGENTS.md`
  - install the shared playbook under a user-scoped pg-logstats agent directory
- Claude Code:
  - install `~/.claude/skills/pg-logstats-triage/SKILL.md`
  - install any supporting playbook files under that skill directory
- Gemini CLI:
  - install `~/.gemini/commands/pg-logstats-triage.toml`
  - install any supporting playbook files under a user-scoped Gemini pg-logstats
    directory

The installed guidance must teach the same workflow:

1. run `pg-logstats readiness --output-format json`
2. inspect `operating_mode`
3. choose only supported workflows
4. respect `verdict`, `allowed_actions`, and `blocked_actions`
5. stop and escalate when the report says evidence is insufficient or the
   database is saturated

## Configuration And Extension Model

V1 should have configuration from the start. Configuration is not only for user
preferences; it is the mechanism that allows fast adaptation for a client
workload without releasing a new `pg-logstats` binary.

Configuration inputs:

- explicit `--config <path>`
- `PG_LOGSTATS_CONFIG`
- default user config at `~/.config/pg-logstats/config.toml`

Precedence:

1. CLI flags
2. `--config <path>`
3. `PG_LOGSTATS_CONFIG`
4. default user config
5. built-in defaults

V1 config should control:

- live-state thresholds used by `running-queries`
- enabled and disabled built-in `suggest-sql` rule IDs
- maximum risk emitted in `next_sql[]`
- whether `omitted_sql[]` is emitted
- per-rule limits
- local external `suggest-sql` rule commands
- user-level install targets for Codex, Claude Code, and Gemini CLI guidance
- query text truncation or redaction limits

V1 should support **external rule commands** for `suggest-sql`. This is the
primary extension point for short-turnaround client work.

External rule commands:

- are configured as an argv array whose first item is the executable path
- can be written in any language
- can add new `suggest-sql` rules without a pg-logstats release
- are invoked with target context JSON on stdin
- return suggested SQL JSON on stdout
- use stderr for diagnostics only
- are trusted local extensions written by someone who understands the workload

V1 should not require Rust code plugins. Code plugins add API, build,
compatibility, and loading complexity. External commands are enough for the
first useful extension point because they allow custom code without embedding a
runtime or inventing a rule DSL.

Example config:

```toml
[database]
dsn = "postgres://app_observer@db.example.com:5432/internal_tools"
connect_timeout_ms = 3000

[running_queries.thresholds]
long_running_query_ms = 120000
waiting_session_count_threshold = 2
idle_in_transaction_count_threshold = 2

[suggest_sql]
max_risk = "bounded"
show_omitted = true
disabled_rules = [
  "query_family.pg_stat_statements.by_query_pattern"
]

[suggest_sql.rules.query_family.pg_stat_activity.by_dimensions]
limit = 50

[suggest_sql.external_rules.client_a]
enabled = true
command = ["python3", "/Users/example/client-a/pg-logstats-rules.py"]
timeout_ms = 2000

[agent_install.codex]
agents_md_path = "/Users/example/AGENTS.md"

[agent_install.claude]
skill_dir = "/Users/example/.claude/skills/pg-logstats-triage"

[agent_install.gemini]
commands_dir = "/Users/example/.gemini/commands"
```

External rule command input:

```json
{
  "schema_version": 1,
  "target_workflow": "top_query_families",
  "target": {
    "finding_id": "query_family:...",
    "kind": "query_family",
    "reason_codes": ["high_total_runtime"],
    "application_name": "invoice-helper",
    "database": "internal_tools",
    "user": "app_user",
    "metrics": {
      "execution_count": 184,
      "total_duration_ms": 91200
    }
  },
  "operating_mode": "log_backed",
  "verdict": "busy",
  "allowed_actions": ["stats_view_reads", "bounded_activity_queries"],
  "blocked_actions": ["explain_analyze", "write_or_admin_action"]
}
```

External rule command output:

```json
{
  "suggestions": [
    {
      "rule_id": "client_a.invoice_helper.active_sessions",
      "label": "Inspect invoice helper sessions",
      "risk": "bounded",
      "action_class": "bounded_activity_queries",
      "sql": "SELECT pid, usename, datname, application_name, state, wait_event_type, wait_event, query_start, query FROM pg_stat_activity WHERE application_name LIKE 'invoice-%' AND state <> 'idle' ORDER BY query_start DESC NULLS LAST LIMIT 50;",
      "reason": "Client A invoice apps share the invoice-* application_name prefix.",
      "required_identifiers": ["application_name"]
    }
  ]
}
```

External command validation:

- command config must include `command`, `enabled`, and `timeout_ms`
- command output must be valid JSON
- every suggestion must include `rule_id`, `label`, `risk`, `action_class`,
  `sql`, and `reason`
- `risk` and `action_class` must use known enum values
- non-zero exit, timeout, or invalid JSON should be reported as `rule_errors[]`
- returned suggestions still pass through pg-logstats policy filtering

V1 should trust external rule commands. It should validate the command contract,
but it does not need a sandbox or a security model for malicious local rule
authors.

Other V1 extension points should also be config-driven:

- verdict thresholds for live-state checks
- agent harness install paths
- query text truncation or redaction limits
- built-in rule enablement and per-rule limits

V1 should not make these externally extensible:

- operating mode names
- top-level report schema
- readiness check names
- finding kinds emitted by built-in workflows
- Rust code loading or binary plugins

## V1 Contract Tables

These tables are normative for implementation. Later narrative and examples
should be read through these tables when there is any ambiguity.

### Evidence Source Support

| Evidence source | V1 readiness status | Notes |
| --- | --- | --- |
| Local supported stderr-style logs | Can satisfy `log_backed` | Requires a parser-recognized format, reachable files, statement text, duration evidence, and session or process identity for correlation. |
| AWS RDS / CloudWatch PostgreSQL logs in a parser-supported text shape | Can satisfy `log_backed` | Requires the same statement, duration, and identity evidence as local stderr-style logs. |
| `csvlog` | Documented target, not V1-ready unless parser support is implemented in the same phase | `readiness` must report `unsupported_log_format` rather than count this as `log_backed` merely because `log_destination` includes `csvlog`. |
| `jsonlog` | Documented target, not V1-ready unless parser support is implemented in the same phase | `readiness` must report `unsupported_log_format` rather than count this as `log_backed` merely because `log_destination` includes `jsonlog`. |
| `pg_stat_activity` | Required for `live_only` and `running-queries` | Must be queryable with enough visibility to inspect cluster-wide activity. |
| `pg_stat_statements` | Required for `live_only` and `running-queries` in V1 | Must be loaded, installed in the target database, and queryable. Aggregate history from this view is non-log history, not log-backed event history. |

### Database Connection Discovery

Commands that need live PostgreSQL checks must resolve a connection target in
this order:

1. `--dsn <postgres-url>`
2. `PG_LOGSTATS_DATABASE_URL`
3. `[database].dsn` in the resolved config file

If no connection target is available:

- `readiness` should still perform any requested static or log-source checks
  and report live checks as `skipped` with reason
  `database_connection_not_configured`
- `readiness` must not choose `live_only`
- `running-queries` must fail with a structured error that identifies
  `database_connection_not_configured`
- `suggest-sql` may still operate from an existing report, because it does not
  execute SQL

The exact PostgreSQL client crate is an implementation choice, but V1 must
document accepted DSN forms, SSL behavior, timeout defaults, and how connection
errors appear in JSON output before `readiness` is considered complete.

### Command Inputs

| Command | Required inputs | Optional inputs | Supported modes |
| --- | --- | --- | --- |
| `pg-logstats agent install --harness codex\|claude\|gemini` | harness | `--config` and configured install path overrides | local tool operation, independent of database mode |
| `pg-logstats readiness` | none | `--dsn`, log input args, `--config` | reports `log_backed`, `live_only`, or `unready` |
| `pg-logstats running-queries` | resolvable database connection | `--config`, threshold overrides if exposed as CLI flags | `live_only`, `log_backed` |
| `pg-logstats top query-families` | supported log input and bounded window | `--config`, `--limit`, source-specific input flags | `log_backed` |
| `pg-logstats errors` | supported log input and bounded window | `--config`, `--limit`, source-specific input flags | `log_backed` |
| `pg-logstats temp-files` | supported log input and bounded window with temp-file evidence | `--config`, `--limit`, source-specific input flags | `log_backed` |
| `pg-logstats suggest-sql --findings-file <path> --rank <n>\|--finding-id <id>` | findings report | `--config` | `log_backed`, `live_only` when report target is supported |
| `pg-logstats suggest-sql --running-queries-file <path> --pid <pid>\|--query-id <id>` | running-queries report | `--config` | `live_only`, `log_backed` |

`suggest-sql` support for `running-queries` reports is part of V1. Without it,
`live_only` would have no concrete follow-up SQL path.

### Shared Triage Report Shape

All machine-readable V1 command output should use this report shape:

| Field | Required | Notes |
| --- | --- | --- |
| `schema_version` | yes | Integer. V1 uses `1`. |
| `workflow` | yes | One of the canonical workflow IDs. |
| `operating_mode` | yes | One of `log_backed`, `live_only`, `unready`. |
| `limitations[]` | yes | Empty when none apply. |
| `verdict` | yes when the command can classify safety | Omit only for `agent install`; use `unknown` when sources are insufficient. |
| `verdict_reasons[]` | yes when `verdict` is present | Empty only when `verdict = clear`. |
| `allowed_actions[]` | yes unless `verdict = unknown` or command has no safety policy | Must use canonical action classes. |
| `blocked_actions[]` | yes unless `verdict = unknown` or command has no safety policy | Must use canonical action classes. |
| `analysis_window` | required for log-window workflows | Omit for `readiness`, `agent install`, and point-in-time live snapshots. |
| `source_summary` | required for evidence-producing workflows | Summarizes log source or live views consulted. |
| `payload` | yes | Command-specific object. |

Command-specific payload keys:

| Workflow | Payload key |
| --- | --- |
| `agent_install` | `agent_install` |
| `readiness` | `readiness` |
| `running_queries` | `running_queries` |
| `top_query_families` | `findings` |
| `errors` | `findings` |
| `temp_files` | `findings` |
| `suggest_sql` | `suggest_sql` |

Examples in this document may show payload fields at the top level for
readability, but implementation tests should assert the canonical report shape or
explicitly document any intentional transition period.

### Canonical Enums

Operating modes:

- `log_backed`
- `live_only`
- `unready`

Workflow IDs:

- `agent_install`
- `readiness`
- `running_queries`
- `top_query_families`
- `errors`
- `temp_files`
- `suggest_sql`

Finding kinds emitted by built-in V1 workflows:

- `query_family`
- `error_class`
- `temp_file`

Verdicts:

- `clear`
- `busy`
- `saturated`
- `unknown`

Risk labels:

- `safe`
- `bounded`
- `expensive`
- `requires_human_approval`

Action classes:

- `system_catalog_reads`
- `bounded_activity_queries`
- `stats_view_reads`
- `text_pattern_stats_search`
- `explain_without_analyze`
- `large_unbounded_selects`
- `explain_analyze`
- `write_or_admin_action`

Suggestion statuses:

- `allowed`
- `blocked_by_verdict`
- `blocked_by_config`
- `blocked_by_policy`
- `omitted_not_enough_context`
- `omitted_unsupported_target`

### Verdict Policy Matrix

| Verdict | Allowed action classes | Blocked action classes | Agent instruction |
| --- | --- | --- | --- |
| `clear` | `system_catalog_reads`, `stats_view_reads`, `bounded_activity_queries`, `text_pattern_stats_search`, `explain_without_analyze` | `large_unbounded_selects`, `explain_analyze`, `write_or_admin_action` | Continue with bounded diagnostic reads. |
| `busy` | `system_catalog_reads`, `stats_view_reads`, `bounded_activity_queries` | `text_pattern_stats_search`, `explain_without_analyze`, `large_unbounded_selects`, `explain_analyze`, `write_or_admin_action` | Keep follow-up narrow and low-impact. |
| `saturated` | none by default | all action classes | Stop adding investigative database load and escalate with the report. |
| `unknown` | omitted | omitted | Do not infer safety; escalate or ask for better evidence. |

Config may only make this matrix more restrictive. It must not allow an action
class that the verdict blocks.

### Built-In Rule Registry Contract

Built-in `suggest-sql` rules must be declared in a central registry with:

- stable `rule_id`
- supported target `workflow`
- supported target `kind`
- required identifiers
- emitted `risk`
- emitted `action_class`
- SQL template or generator name
- attribution note or reference back to the workflow attribution map

V1 built-in rule IDs should use these prefixes:

- `query_family.pg_stat_statements.*`
- `query_family.pg_stat_activity.*`
- `running_query.pg_stat_activity.*`
- `running_query.blocking.*`
- `error_class.pg_stat_activity.*`
- `temp_file.pg_stat_database.*`
- `temp_file.pg_stat_statements.*`

### Config Schema Contract

Configuration must be deserialized into typed structs. Unknown top-level keys
should be warnings in V1, not hard errors. Unknown keys below known sections
should be warnings unless they look like misspelled safety fields such as
`max_risk`, `disabled_rules`, or threshold names.

Minimum V1 sections:

```toml
[database]
dsn = "postgres://user@host:5432/dbname"
connect_timeout_ms = 3000

[running_queries.thresholds]
long_running_query_ms = 120000
waiting_session_count_threshold = 2
idle_in_transaction_count_threshold = 2

[suggest_sql]
max_risk = "bounded"
show_omitted = true
disabled_rules = []

[suggest_sql.rules.<rule_id>]
enabled = true
limit = 20

[suggest_sql.external_rules.<name>]
enabled = true
command = ["/absolute/path/to/program", "--flag"]
timeout_ms = 2000

[agent_install.codex]
agents_md_path = "/Users/example/AGENTS.md"
playbook_dir = "/Users/example/.config/pg-logstats/agents"
```

External commands should be configured as an argv array, not a shell string, so
that paths and arguments are unambiguous. If V1 also accepts a legacy string
form, the docs must state how it is split and what quoting rules apply.

### External Rule Command ABI

The external command ABI is versioned separately from the top-level report
schema:

- input field `schema_version = 1`
- output field `schema_version = 1`
- unknown input fields must be ignored by rule commands
- unknown output fields must be preserved only if they fit under a documented
  `metadata` object; otherwise they are ignored with a warning
- stdout must contain one JSON object and should stay under the configured
  `max_stdout_bytes`
- stderr is diagnostic text only and should be captured into `rule_errors[]`
  when the command fails
- commands run with the current process environment plus
  `PG_LOGSTATS_EXTERNAL_RULE=1`
- commands receive no database credentials unless the author explicitly puts
  them in the command environment outside pg-logstats
- pg-logstats applies redaction and query truncation before building the target
  context sent to external commands

V1 trusts local rule authors, but every returned suggestion still passes through
schema validation, verdict policy, config risk ceiling, and blocked-action
filtering before it can appear in `next_sql[]`.

### Readiness Checks By Mode

The implementation should make mode selection deterministic.

#### `log_backed`

Required checks:

- supported log source exists
- logging configuration can produce statement and duration evidence
- parser can identify a supported log format

If all pass:

- `operating_mode = log_backed`

#### `live_only`

Required checks:

- `track_activities = on`
- `shared_preload_libraries` contains `pg_stat_statements`
- `compute_query_id` is enabled
- `pg_stat_statements` extension exists
- `pg_stat_activity` and `pg_stat_statements` are both queryable
- caller has cluster-wide stats visibility

If all pass and `log_backed` fails:

- `operating_mode = live_only`

#### `unready`

If both `log_backed` and `live_only` checks fail:

- `operating_mode = unready`

## CLI Surface

V1 introduces or sharpens the following command surface.

### `pg-logstats agent install`

Purpose:

- install harness-specific skills or playbooks into the user-level configuration
  location for a supported agent
- honor configured install target overrides

Required inputs:

- `--harness codex|claude|gemini`

Optional inputs:

- `--config <path>`
- `--dry-run` to report intended writes without modifying files
- `--status` to report whether the selected harness appears installed

Side effects:

- writes or updates harness-specific guidance files in the user-level location
- installs the shared pg-logstats playbook content used by that harness
- uses `[agent_install.*]` config paths when present

Output schema:

- `harness`
- `install_location`
- `files_written[]`
- `files_updated[]`
- `status`

Harness-specific installation targets:

- `codex`
  - managed block in user-scoped `AGENTS.md`
- `claude`
  - `~/.claude/skills/pg-logstats-triage/SKILL.md`
- `gemini`
  - `~/.gemini/commands/pg-logstats-triage.toml`

The implementation should make the install command idempotent. Re-running the
command should update the managed pg-logstats content rather than duplicate it.
Managed files or blocks must use stable begin/end markers, and the command
should report whether it wrote, updated, skipped, or would update each file.

### `pg-logstats readiness`

Purpose:

- detect operating mode before deeper investigation
- report readiness of database evidence and `pg-logstats` capabilities

Required inputs:

- none

Sources:

- PostgreSQL configuration via `SHOW`
- `pg_extension`
- `pg_stat_activity`
- `pg_stat_statements`
- direct log-source checks
- local filesystem checks for harness-specific guidance artifacts

Optional inputs:

- `--dsn <postgres-url>` or the equivalent configured database connection
- log input arguments used to prove a supported log source is reachable
- `--config <path>`

Implementation checks:

- collect all required `SHOW` values
- check required extension and role membership
- attempt lightweight probe queries against required views
- determine whether a supported log source is reachable

Reference probe SQL:

```sql
SHOW log_destination;
SHOW log_line_prefix;
SHOW log_duration;
SHOW log_min_duration_statement;
SHOW log_temp_files;
SHOW track_activities;
SHOW shared_preload_libraries;
SHOW compute_query_id;
```

```sql
SELECT extname
FROM pg_extension
WHERE extname = 'pg_stat_statements';
```

```sql
SELECT pg_has_role(current_user, 'pg_read_all_stats', 'member') AS has_pg_read_all_stats;
```

```sql
SELECT
  pid,
  datname,
  usename,
  application_name,
  state,
  wait_event_type,
  wait_event,
  query_id,
  query
FROM pg_stat_activity
LIMIT 1;
```

```sql
SELECT
  queryid,
  query,
  calls,
  total_exec_time,
  mean_exec_time
FROM pg_stat_statements
ORDER BY total_exec_time DESC
LIMIT 1;
```

Mode-determination rules:

- choose `log_backed` only if all required logging checks pass and a supported
  log source is reachable
- choose `live_only` only if `log_backed` is unavailable and all required
  `pg_stat_activity` / `pg_stat_statements` checks pass
- choose `unready` otherwise

Implementation notes:

- `shared_preload_libraries` should be parsed as a comma-separated setting and
  matched case-sensitively against `pg_stat_statements`
- `log_destination` should be parsed as a comma-separated setting
- `log_min_duration_statement = -1` means disabled and does not satisfy the
  log-backed duration requirement
- `pg_has_role(current_user, 'pg_read_all_stats', 'member') = true` satisfies
  the monitoring-role requirement; superuser also satisfies it
- probe-query failure should be recorded as a failed check, not collapsed into a
  generic connection error
- missing database connection should record live checks as `skipped` with reason
  `database_connection_not_configured`, not as passed or failed PostgreSQL
  checks

Output schema:

- `operating_mode`
- `database_readiness`
- `agent_readiness`
- `required_checks`
- `failed_checks`
- limitations
- recommended next commands

Example fields:

- `database_readiness.mode_candidate`
- `database_readiness.checks.log_duration`
- `database_readiness.checks.pg_stat_statements_extension`
- `agent_readiness.codex.installed`
- `agent_readiness.claude.installed`
- `agent_readiness.gemini.installed`

Illustrative JSON:

```json
{
  "schema_version": 1,
  "workflow": "readiness",
  "operating_mode": "live_only",
  "database_readiness": {
    "mode_candidate": "live_only",
    "checks": {
      "log_destination": {
        "status": "failed",
        "value": "stderr",
        "reason": "supported_log_source_unreachable"
      },
      "log_duration": {
        "status": "failed",
        "value": "off"
      },
      "log_min_duration_statement": {
        "status": "failed",
        "value": "-1"
      },
      "track_activities": {
        "status": "passed",
        "value": "on"
      },
      "shared_preload_libraries": {
        "status": "passed",
        "value": "pg_stat_statements"
      },
      "compute_query_id": {
        "status": "passed",
        "value": "auto"
      },
      "pg_stat_statements_extension": {
        "status": "passed",
        "value": true
      },
      "pg_read_all_stats": {
        "status": "passed",
        "value": true
      },
      "pg_stat_activity_probe": {
        "status": "passed"
      },
      "pg_stat_statements_probe": {
        "status": "passed"
      }
    }
  },
  "agent_readiness": {
    "codex": {
      "status": "passed",
      "installed": true,
      "install_location": "~/AGENTS.md"
    },
    "claude": {
      "status": "failed",
      "installed": false,
      "install_location": "~/.claude/skills/pg-logstats-triage/SKILL.md"
    },
    "gemini": {
      "status": "failed",
      "installed": false,
      "install_location": "~/.gemini/commands/pg-logstats-triage.toml"
    }
  },
  "failed_checks": [
    "supported_log_source_unreachable",
    "log_duration_disabled"
  ],
  "limitations": [
    "historical_log_triage_unavailable",
    "event_level_evidence_unavailable"
  ],
  "recommended_next_commands": [
    "pg-logstats running-queries --output-format json"
  ]
}
```

### `pg-logstats running-queries`

Purpose:

- provide live-state preflight for current database pressure

Sources:

- `pg_stat_activity`
- `pg_stat_statements`

Required inputs:

- resolvable database connection from `--dsn`, `PG_LOGSTATS_DATABASE_URL`, or
  `[database].dsn`
- `pg_stat_activity`
- `pg_stat_statements`
- `pg_read_all_stats` visibility or equivalent superuser access

Information extracted from `pg_stat_activity`:

- `datname`
- `pid`
- `leader_pid`
- `usename`
- `application_name`
- `client_addr`
- `backend_start`
- `xact_start`
- `query_start`
- `state_change`
- `wait_event_type`
- `wait_event`
- `state`
- `query_id`
- `query`
- `backend_type`

Information extracted from `pg_stat_statements`:

- `queryid`
- `query`
- `calls`
- `total_exec_time`
- `mean_exec_time`
- `rows`
- `shared_blks_hit`
- `shared_blks_read`
- `temp_blks_read`
- `temp_blks_written`

Join strategy:

- join active `pg_stat_activity.query_id` to `pg_stat_statements.queryid` when
  available
- if `query_id` is unavailable for a backend, surface the active-session record
  without statement-history enrichment

Reference query shape:

```sql
SELECT
  a.datname,
  a.pid,
  a.leader_pid,
  a.usename,
  a.application_name,
  a.client_addr,
  a.backend_start,
  a.xact_start,
  a.query_start,
  a.state_change,
  a.wait_event_type,
  a.wait_event,
  a.state,
  a.query_id,
  a.query,
  a.backend_type,
  s.calls,
  s.total_exec_time,
  s.mean_exec_time,
  s.rows,
  s.shared_blks_hit,
  s.shared_blks_read,
  s.temp_blks_read,
  s.temp_blks_written
FROM pg_stat_activity AS a
LEFT JOIN pg_stat_statements AS s
  ON a.query_id = s.queryid
WHERE a.backend_type = 'client backend';
```

Implementation notes:

- compute `duration_ms` as the difference between the collection timestamp and
  `query_start` for `state = 'active'`
- retain rows for non-active sessions only when they contribute to triage, such
  as `idle in transaction` or waiting sessions
- exclude the `pg-logstats` session itself when possible
- truncate or omit `query_text` only when PostgreSQL permissions or configured
  truncation prevent full visibility
- preserve nullability for `query_id` and `statement_history`

Output schema:

- top-level `verdict`
- `operating_mode`
- `sources`
- active query summary
- `active_sessions[]`
- `blocking_signals[]`
- blocked or allowed action classes

Per-session output fields:

- `pid`
- `database`
- `user`
- `application_name`
- `state`
- `wait_event_type`
- `wait_event`
- `query_start`
- `duration_ms`
- `query_id`
- `query_text`
- `statement_history` when joined to `pg_stat_statements`

Verdict inputs:

- count of long-running active queries
- count of sessions in `idle in transaction`
- presence of wait events
- concentration of runtime or calls for the active query family when
  `pg_stat_statements` data is available

Initial V1 verdict rules:

- `saturated`
  - at least one active client backend exceeds a configured long-running-query
    threshold and at least one additional session is waiting, blocked, or idle
    in transaction
- `busy`
  - one or more active client backends exceed the configured long-running-query
    threshold
  - or multiple client backends are waiting on lock or IO-related events
  - or multiple sessions are `idle in transaction`
- `clear`
  - none of the `busy` or `saturated` conditions apply
- `unknown`
  - required live-state sources are unavailable for this command

V1 configuration inputs that should be explicit in implementation:

- `long_running_query_ms`
- `waiting_session_count_threshold`
- `idle_in_transaction_count_threshold`

Supported modes:

- `live_only`
- `log_backed`

Illustrative JSON:

```json
{
  "schema_version": 1,
  "workflow": "running_queries",
  "operating_mode": "live_only",
  "sources": {
    "pg_stat_activity": "available",
    "pg_stat_statements": "available"
  },
  "verdict": "busy",
  "verdict_reasons": [
    "long_running_queries_present",
    "waiting_sessions_present"
  ],
  "allowed_actions": [
    "system_catalog_reads",
    "bounded_activity_queries"
  ],
  "blocked_actions": [
    "large_unbounded_selects",
    "explain_analyze"
  ],
  "active_query_summary": {
    "active_session_count": 3,
    "waiting_session_count": 2,
    "idle_in_transaction_count": 1,
    "long_running_query_count": 1
  },
  "blocking_signals": [
    {
      "kind": "wait_event",
      "wait_event_type": "Lock",
      "count": 2
    }
  ],
  "active_sessions": [
    {
      "pid": 4812,
      "database": "internal_tools",
      "user": "app_user",
      "application_name": "invoice-helper",
      "state": "active",
      "wait_event_type": "Lock",
      "wait_event": "transactionid",
      "query_start": "2026-06-05T10:02:00Z",
      "duration_ms": 184230,
      "query_id": 918273645,
      "query_text": "select * from invoices where workspace_id = $1 order by created_at desc",
      "statement_history": {
        "calls": 184,
        "total_exec_time": 91200.0,
        "mean_exec_time": 495.65,
        "rows": 1840,
        "shared_blks_hit": 129381,
        "shared_blks_read": 918,
        "temp_blks_read": 0,
        "temp_blks_written": 0
      }
    }
  ]
}
```

### `pg-logstats top query-families`

Purpose:

- rank suspicious query families in a bounded historical window

Sources:

- supported PostgreSQL logs only

Information extracted:

- timestamp
- session or process identity
- user
- database
- application name
- statement text
- duration
- query family normalization inputs

Output schema:

- ranked findings
- app, user, and database attribution when known
- evidence handles
- risk-labeled next SQL

Per-finding fields:

- `finding_id`
- `kind`
- `rank`
- `title`
- `reason_codes`
- `application_name`
- `database`
- `user`
- `normalized_sql`
- `metrics.execution_count`
- `metrics.total_duration_ms`
- `metrics.mean_duration_ms`
- `metrics.p95_duration_ms`
- `evidence.sample_event_refs`
- `next_sql[]`

Supported modes:

- `log_backed`

### `pg-logstats errors`

Purpose:

- surface grouped error and event triage in a bounded historical window

Sources:

- supported PostgreSQL logs only

Information extracted:

- timestamp
- severity
- SQLSTATE when available
- message text
- statement text when present
- user, database, application name

Output schema:

- grouped `findings[]` by SQLSTATE or normalized error text
- representative evidence handles
- risk-labeled next SQL when applicable

Supported modes:

- `log_backed`

### `pg-logstats temp-files`

Purpose:

- surface temp-file-driven resource pressure in a bounded historical window

Sources:

- supported PostgreSQL logs with `log_temp_files` enabled

Information extracted:

- timestamp
- temp-file size
- user, database, application name
- session or process identity
- nearby statement or duration correlation when available

Output schema:

- grouped `findings[]`
- bytes written
- event counts
- app, user, and database attribution
- evidence handles
- risk-labeled next SQL when applicable

Supported modes:

- `log_backed`

### `pg-logstats suggest-sql`

Purpose:

- emit bounded, risk-labeled follow-up SQL for a finding or live-state path

Sources:

- finding report from another `pg-logstats` workflow
- live-state path description from `running-queries`
- built-in rule registry
- external rule commands from config

Inputs:

- `--findings-file <path>` plus either `--rank <n>` or `--finding-id <id>`
- `--running-queries-file <path>` plus `--pid <pid>` or `--query-id <id>`

V1 rule lifecycle:

`pg-logstats` should use the same conceptual lifecycle for built-in rules and
external rule commands. Built-in rules implement the lifecycle as internal
function calls. External rule commands implement their own matching and
candidate generation behind a single process invocation.

1. Discover rule sources:
   - load the built-in rule registry
   - load configured external rule commands
   - apply config for disabled built-in rules and disabled external commands
2. Build target context:
   - selected finding or live-state target
   - `workflow`
   - `operating_mode`
   - `verdict`
   - `allowed_actions`
   - `blocked_actions`
   - target kind, dimensions, metrics, and evidence handles
3. Evaluate and generate candidates:
   - built-in rules evaluate applicability and generate candidates in-process
   - each enabled external command receives the same target context JSON on stdin
   - each external command returns zero or more candidate suggestions on stdout
4. Normalize candidates:
   - validate required fields
   - attach `rule_source`
   - attach external command identity when applicable
   - convert failures into `rule_errors[]`
5. Apply pg-logstats policy:
   - enforce `max_risk`
   - enforce `blocked_actions`
   - classify candidates into `next_sql[]` or `omitted_sql[]`
   - sort runnable suggestions by risk and specificity

This lifecycle avoids a custom rule DSL while keeping `pg-logstats` responsible
for the final output policy.

V1 command algorithm:

1. Load the source report and validate `schema_version`.
2. Select exactly one target finding or live-state target.
3. Build the target context described above.
4. Derive common identifiers for built-in SQL generation:
   - `query_id` or `queryid`
   - `application_name`
   - `database`
   - `user`
   - `pid`
   - SQLSTATE or normalized error text
5. Run the rule lifecycle.
6. Mark candidates as blocked when their `action_class` appears in
   `blocked_actions`, or when their risk maps to a blocked action.
7. Keep blocked candidates only as metadata when useful for escalation.
8. Sort candidates by risk, then specificity:
   - exact `pid`
   - exact `query_id`
   - exact `application_name` / database / user
   - aggregate database-level view

Validation checks:

- source report must be valid JSON with a supported schema version
- target selector must resolve to exactly one target
- target kind must be supported by `suggest-sql`
- built-in SQL templates must use only read-only `SELECT` statements
- template values must be SQL-literal escaped
- any pattern match must use an explicit `LIMIT`
- generated SQL must include enough predicates to match the declared risk label
- templates with missing required identifiers must be omitted instead of emitted
  with empty predicates
- if all runnable candidates are blocked, `next_sql[]` should be empty and
  `omitted_sql[]` should explain why
- external commands must complete within their configured timeout
- external command stdout must pass suggestion schema validation before any
  returned suggestion is used

`suggest-sql` must never generate:

- DDL
- DML
- `VACUUM`
- `ANALYZE`
- `EXPLAIN ANALYZE`
- unbounded user-table queries
- queries that require table names inferred from arbitrary SQL parsing

This "never generate" list applies to built-in rules. V1 external rule commands
are trusted local extensions, so the implementation should warn on suspicious
returned SQL but not treat safety validation as a security boundary.

Risk model:

- `safe`
  - reads only PostgreSQL statistics or catalog views
  - uses exact identifiers such as `pid`, `query_id`, `queryid`, database, user,
    or application name
  - has a hard `LIMIT` when returning rows
- `bounded`
  - reads only PostgreSQL statistics or catalog views
  - has filters and limits, but relies on broader predicates such as
    `application_name`, database, user, or query text pattern
- `expensive`
  - still read-only, but may scan larger statistics views, perform text pattern
    matching, or produce more rows
  - should be omitted when `large_unbounded_selects` or equivalent action class
    is blocked
- `requires_human_approval`
  - valid diagnostic SQL exists, but V1 should not emit it as runnable SQL for
    agents
  - examples include `EXPLAIN`, `EXPLAIN ANALYZE`, lock termination, index
    creation, or table-specific inspection

Risk is assigned from the SQL source and predicate shape, not from the severity
of the finding. A severe finding can still have only `safe` follow-up SQL.

Config can reduce the emitted risk ceiling. For example, `max_risk = "safe"`
means `bounded`, `expensive`, and `requires_human_approval` candidates move to
`omitted_sql[]`.

Action classes:

- `system_catalog_reads`
- `bounded_activity_queries`
- `stats_view_reads`
- `text_pattern_stats_search`
- `explain_without_analyze`
- `large_unbounded_selects`
- `explain_analyze`
- `write_or_admin_action`

Risk-to-policy mapping:

- `safe`
  - allowed when `system_catalog_reads`, `stats_view_reads`, or
    `bounded_activity_queries` are allowed
- `bounded`
  - allowed when its exact `action_class` is allowed and the SQL has a hard
    `LIMIT`
- `expensive`
  - blocked when `large_unbounded_selects`, `text_pattern_stats_search`, or its
    exact `action_class` is blocked
- `requires_human_approval`
  - never emitted in `next_sql[]`; record only in `omitted_sql[]`

Output schema:

- `target_workflow`
- `target_identifier`
- `operating_mode`
- `verdict`
- `next_sql[]`
- `omitted_sql[]`
- `rule_sources`
- `rule_errors[]`

Per-item fields:

- `label`
- `rule_id`
- `rule_source`
- `external_command`
- `risk`
- `action_class`
- `status`
- `sql`
- `reason`
- `required_identifiers[]`

`rule_source` values:

- `built_in`
- `external_command`

`rule_errors[]` fields:

- `rule_source`
- `external_command`
- `status`
- `message`

`status` values:

- `allowed`
- `blocked_by_verdict`
- `omitted_not_enough_context`

Supported modes:

- `log_backed`
- `live_only`

#### Query-family templates

Use when the target kind is `query_family` or `slow_query_regression`.

Exact `queryid` lookup:

```sql
SELECT
  queryid,
  calls,
  total_exec_time,
  mean_exec_time,
  min_exec_time,
  max_exec_time,
  rows,
  shared_blks_hit,
  shared_blks_read,
  temp_blks_read,
  temp_blks_written,
  query
FROM pg_stat_statements
WHERE queryid = :queryid;
```

Risk:

- `safe`

Action class:

- `stats_view_reads`

Active sessions by exact dimensions:

```sql
SELECT
  pid,
  usename,
  datname,
  application_name,
  state,
  wait_event_type,
  wait_event,
  query_start,
  query_id,
  query
FROM pg_stat_activity
WHERE datname = :database
  AND usename = :user
  AND application_name = :application_name
  AND state <> 'idle'
ORDER BY query_start DESC NULLS LAST
LIMIT 20;
```

Risk:

- `safe` when at least one exact dimension is present
- `bounded` when only database or user is present

Action class:

- `bounded_activity_queries`

The implementation should include only predicates backed by present dimensions.
If no database, user, or application name is present, omit this candidate.

Fallback `pg_stat_statements` text search:

```sql
SELECT
  queryid,
  calls,
  total_exec_time,
  mean_exec_time,
  rows,
  query
FROM pg_stat_statements
WHERE query ILIKE :query_pattern
ORDER BY total_exec_time DESC
LIMIT 20;
```

Risk:

- `bounded`

Action class:

- `text_pattern_stats_search`

Use this only when `queryid` is unavailable and the normalized SQL can produce a
non-empty pattern.

#### Running-query templates

Use when the target is selected from `running-queries`.

Exact backend lookup:

```sql
SELECT
  pid,
  usename,
  datname,
  application_name,
  client_addr,
  state,
  wait_event_type,
  wait_event,
  xact_start,
  query_start,
  state_change,
  query_id,
  query
FROM pg_stat_activity
WHERE pid = :pid;
```

Risk:

- `safe`

Action class:

- `bounded_activity_queries`

Blocking context:

```sql
SELECT
  blocked.pid AS blocked_pid,
  blocked.application_name AS blocked_application_name,
  blocked.query AS blocked_query,
  blocker.pid AS blocker_pid,
  blocker.application_name AS blocker_application_name,
  blocker.state AS blocker_state,
  blocker.query AS blocker_query
FROM pg_stat_activity AS blocked
JOIN LATERAL unnest(pg_blocking_pids(blocked.pid)) AS blocking_pid ON true
JOIN pg_stat_activity AS blocker ON blocker.pid = blocking_pid
WHERE blocked.pid = :pid;
```

Risk:

- `safe`

Action class:

- `bounded_activity_queries`

#### Error-class templates

Use when the target kind is `error_class`.

Active sessions for the same app/database/user:

```sql
SELECT
  pid,
  usename,
  datname,
  application_name,
  state,
  wait_event_type,
  wait_event,
  query_start,
  query
FROM pg_stat_activity
WHERE datname = :database
  AND usename = :user
  AND application_name = :application_name
  AND state <> 'idle'
ORDER BY query_start DESC NULLS LAST
LIMIT 20;
```

Risk:

- `safe` with exact app/database/user
- `bounded` with partial dimensions

Action class:

- `bounded_activity_queries`

The implementation should include only predicates backed by present dimensions.
If no database, user, or application name is present, omit this candidate.

V1 should not generate SQL for historical SQLSTATE counts from PostgreSQL unless
there is a concrete source such as logs. SQLSTATE history is a log-backed
finding, not a live catalog query.

#### Temp-file templates

Use when the target kind is `temp_file`.

Database temp-file counters:

```sql
SELECT
  datname,
  temp_files,
  temp_bytes
FROM pg_stat_database
WHERE datname = :database;
```

Risk:

- `safe`

Action class:

- `stats_view_reads`

Query families with temp block activity:

```sql
SELECT
  queryid,
  calls,
  total_exec_time,
  mean_exec_time,
  temp_blks_read,
  temp_blks_written,
  query
FROM pg_stat_statements
WHERE temp_blks_read > 0
   OR temp_blks_written > 0
ORDER BY temp_blks_written DESC, temp_blks_read DESC
LIMIT 20;
```

Risk:

- `bounded`

Action class:

- `stats_view_reads`

If the temp-file finding has `queryid`, add `WHERE queryid = :queryid` and label
the candidate `safe`.

#### Omitted suggestions

When V1 intentionally does not emit runnable SQL, it should record the omission:

```json
{
  "label": "Explain the query plan",
  "risk": "requires_human_approval",
  "action_class": "explain_without_analyze",
  "status": "blocked_by_verdict",
  "reason": "The current verdict blocks plan inspection from agent-driven flow."
}
```

This lets the agent produce a better escalation report without running unsafe or
underspecified SQL.

Illustrative output:

```json
{
  "schema_version": 1,
  "workflow": "suggest_sql",
  "target_workflow": "top_query_families",
  "target_identifier": "query_family:app=invoice-helper:sql=...",
  "operating_mode": "log_backed",
  "verdict": "busy",
  "rule_sources": ["built_in"],
  "next_sql": [
    {
      "label": "Inspect statement statistics for the exact query family",
      "rule_id": "query_family.pg_stat_statements.by_queryid",
      "rule_source": "built_in",
      "risk": "safe",
      "action_class": "stats_view_reads",
      "status": "allowed",
      "sql": "SELECT queryid, calls, total_exec_time, mean_exec_time, min_exec_time, max_exec_time, rows, shared_blks_hit, shared_blks_read, temp_blks_read, temp_blks_written, query FROM pg_stat_statements WHERE queryid = 918273645;",
      "reason": "The finding includes queryid, so this is an exact stats-view lookup.",
      "required_identifiers": ["queryid"]
    },
    {
      "label": "Find current active sessions for the same app and database",
      "rule_id": "query_family.pg_stat_activity.by_dimensions",
      "rule_source": "built_in",
      "risk": "safe",
      "action_class": "bounded_activity_queries",
      "status": "allowed",
      "sql": "SELECT pid, usename, datname, application_name, state, wait_event_type, wait_event, query_start, query_id, query FROM pg_stat_activity WHERE datname = 'internal_tools' AND usename = 'app_user' AND application_name = 'invoice-helper' AND state <> 'idle' ORDER BY query_start DESC NULLS LAST LIMIT 20;",
      "reason": "The query is bounded to the app, database, and user dimensions from the finding.",
      "required_identifiers": ["database", "user", "application_name"]
    }
  ],
  "omitted_sql": [
    {
      "label": "Explain the query plan",
      "rule_id": "query_family.explain.without_analyze",
      "rule_source": "built_in",
      "risk": "requires_human_approval",
      "action_class": "explain_without_analyze",
      "status": "blocked_by_verdict",
      "reason": "Plan inspection is intentionally left to a human-approved step in V1."
    }
  ]
}
```

## Output Contract

All machine-readable V1 workflows should use the shared triage report shape defined
in the V1 Contract Tables section. Workflow-specific data belongs under the
command-specific `payload` key listed there.

Illustrative shape:

```json
{
  "schema_version": 1,
  "workflow": "top_query_families",
  "operating_mode": "log_backed",
  "limitations": [],
  "analysis_window": {
    "since": "2026-06-05T10:00:00Z",
    "until": "2026-06-05T10:30:00Z"
  },
  "source_summary": {
    "kind": "local_stderr",
    "entries_scanned": 18420
  },
  "verdict": "busy",
  "verdict_reasons": ["long_running_queries_present"],
  "allowed_actions": [
    "system_catalog_reads",
    "bounded_activity_queries"
  ],
  "blocked_actions": ["large_unbounded_selects", "explain_analyze"],
  "payload": {
    "findings": []
  }
}
```

### Degraded Output Requirements

`pg-logstats readiness` is the authoritative mode and capability report.

It should:

- state the active operating mode explicitly
- enumerate the important limitations of that mode
- recommend the next best supported commands

Other commands do not need to repeat the full degraded explanation every time.
They should carry only enough mode metadata to prevent misuse when the current
mode changes what the command can honestly claim.

Example:

```json
{
  "schema_version": 1,
  "workflow": "readiness",
  "operating_mode": "live_only",
  "limitations": [
    "historical_log_triage_unavailable",
    "query_family_runtime_ranking_unavailable"
  ],
  "payload": {
    "readiness": {
      "recommended_next_commands": [
        "pg-logstats running-queries --output-format json"
      ]
    }
  }
}
```

## Attribution References

The implementation should align its checks and field selection with the official
PostgreSQL documentation and should use pgBadger as prior art for established
log-backed triage categories:

- pgBadger feature docs for prior-art report categories:
  <https://access.crunchydata.com/documentation/pgbadger/latest/>
- pgBadger report examples for implementation inspiration:
  <https://pgbadger.darold.net/#reports>
- pgBadger source, logging prerequisites:
  <https://github.com/darold/pgbadger/blob/master/pgbadger#L1>
- pgBadger source, temporary-file reports:
  <https://github.com/darold/pgbadger/blob/master/pgbadger#L1158-L1187>
- pgBadger source, temporary-file activity charts:
  <https://github.com/darold/pgbadger/blob/master/pgbadger#L804-L832>
- pgBadger source, top-query reports:
  <https://github.com/darold/pgbadger/blob/master/pgbadger#L1255-L1272>
- pgBadger source, normalized slow-query reports:
  <https://github.com/darold/pgbadger/blob/master/pgbadger#L1386-L1389>
- pgBadger source, error class distribution:
  <https://github.com/darold/pgbadger/blob/master/pgbadger#L1609-L1623>
- pgBadger source, frequent errors/events:
  <https://github.com/darold/pgbadger/blob/master/pgbadger#L1665-L1671>
- `pg_stat_activity`: PostgreSQL docs, Monitoring Database Activity,
  <https://www.postgresql.org/docs/current/monitoring-stats.html>
- `pg_stat_statements`: PostgreSQL docs,
  <https://www.postgresql.org/docs/current/pgstatstatements.html>
- `log_temp_files`: PostgreSQL docs,
  <https://www.postgresql.org/docs/current/runtime-config-logging.html#GUC-LOG-TEMP-FILES>
- `pg_blocking_pids`: PostgreSQL docs,
  <https://www.postgresql.org/docs/current/functions-info.html>
- predefined monitoring roles such as `pg_read_all_stats`: PostgreSQL docs,
  <https://www.postgresql.org/docs/current/predefined-roles.html>
- logging configuration: PostgreSQL docs, Error Reporting and Logging,
  <https://www.postgresql.org/docs/current/runtime-config-logging.html>

## Phased Implementation

The implementation should proceed in phases, but all phases contribute to the
same V1 design. The repo already has partial event, correlation, finding,
`top query-families`, `slow-queries diff`, and basic `suggest-sql` support, so
the plan below focuses on bringing existing pieces into this V1 contract rather
than rebuilding the foundations from scratch.

Every phase must include tests and docs as part of the implementation. Tests
should cover new policy logic, JSON schema shape, CLI behavior when relevant,
and deterministic ranking or reason-code behavior. Docs should cover user-facing
behavior, degraded behavior, unsupported modes, and attribution references for
each built-in workflow or rule.

`slow-queries diff` is not the V1 spine for this internal AI-app triage story.
Treat it as an existing experimental workflow unless a later decision explicitly
promotes it into the shared report contract.

### Phase 1: Config And Shared Report Contract

Deliver:

- typed config structs
- config discovery and precedence: CLI flags, `--config`, `PG_LOGSTATS_CONFIG`,
  default user config, built-in defaults
- `--config <path>` global flag
- `PG_LOGSTATS_CONFIG`
- default user config path
- canonical enums for operating modes, workflow IDs, verdicts, risk labels,
  action classes, suggestion statuses, finding kinds, and check statuses
- shared `PgTriageReport` structs
- JSON serialization tests for the shared report shape
- transition adapter for current `FindingSet` output where needed

Docs:

- config file discovery and precedence
- minimal config example
- `PgTriageReport` example
- canonical enum tables

Acceptance criteria:

- config precedence is deterministic and tested
- unknown config behavior is documented and tested
- at least one existing command can emit report-shaped JSON output behind the
  new model or through an explicit transition path

### Phase 2: Readiness And Mode Detection

Deliver:

- `pg-logstats readiness`
- database connection discovery from `--dsn`, `PG_LOGSTATS_DATABASE_URL`, or
  `[database].dsn`
- lightweight PostgreSQL probe runner
- log-source reachability checks
- parser-supported evidence checks for `log_backed`
- `live_only` checks for `pg_stat_activity`, `pg_stat_statements`,
  `compute_query_id`, and monitoring-role visibility
- agent guidance artifact detection
- operating mode detection: `log_backed`, `live_only`, `unready`
- machine-readable limitations and next-command hints

Docs:

- readiness-first workflow
- supported and unsupported evidence sources
- database permissions and connection assumptions
- examples for `log_backed`, `live_only`, and `unready`

Acceptance criteria:

- missing database connection records live checks as skipped, not passed
- `csvlog` and `jsonlog` do not satisfy `log_backed` until parser support exists
- readiness can report useful log-backed status without live DB access when log
  input is available
- degraded mode is explicit and parseable

### Phase 3: Log-Backed Query Family Triage

Deliver:

- `top query-families` aligned to `log_backed` mode
- shared triage report output
- bounded historical window support
- source summary
- app, user, and database attribution surfaced in findings
- missing attribution represented explicitly
- stable finding identifiers and evidence handles
- deterministic reason codes
- attribution references to pgBadger top-query prior art and PostgreSQL docs

Docs:

- bounded-window query-family triage
- required log settings and parser-supported formats
- how missing app/user/database attribution appears
- prior-art references engineers used

Acceptance criteria:

- ranked output is deterministic
- output follows the report contract
- findings degrade honestly when attribution is missing
- unsupported modes fail clearly

### Phase 4: Suggest-SQL Framework And Policy Engine

Deliver:

- source report loading and schema validation
- target selection by `--finding-id`, `--rank`, `--pid`, or `--query-id`
- rule registry abstraction
- built-in rule source interface
- external command rule source interface
- target context JSON for rule evaluation
- candidate normalization
- SQL suggestion schema with `rule_id`, `rule_source`, `risk`,
  `action_class`, `status`, `reason`, and `required_identifiers[]`
- verdict/action/risk policy filtering
- `next_sql[]`
- `omitted_sql[]`
- `rule_errors[]`
- config controls for rule enablement, risk ceiling, omitted suggestions,
  per-rule limits, external command timeout, and stdout limits

Docs:

- `suggest-sql` command usage
- rule lifecycle
- risk labels
- action classes
- policy matrix
- external rule command input and output schemas
- external command timeout and failure behavior
- examples for allowed, omitted, blocked, and failed external suggestions

Acceptance criteria:

- SQL suggestions are never unlabeled
- blocked classes are machine-readable
- config can only make policy stricter
- invalid source reports fail with structured errors
- external rule command suggestions can be emitted with
  `rule_source = "external_command"`
- external rule command failures are reported in `rule_errors[]`
- no real built-in SQL rule needs to ship in this phase beyond test fixtures or
  a minimal internal fixture rule

### Phase 5: Initial Built-In Suggest-SQL Rules

Deliver query-family rules:

- exact `pg_stat_statements` lookup by `queryid`
- active sessions by app, database, and user dimensions
- bounded `pg_stat_statements` text search fallback when `queryid` is missing

Do not implement running-query, error-class, or temp-file rules in this phase
unless their source report workflows already exist. Those rule families should
ship with the workflows that create their target reports.

Docs:

- built-in query-family rule catalog with stable rule IDs
- required identifiers per rule
- risk and action class per rule
- attribution for the query-family rule family

Acceptance criteria:

- every query-family built-in rule has tests for allowed output
- every query-family built-in rule has tests for missing identifiers
- SQL literal escaping is tested
- risk/action labels match the rule catalog
- blocked and omitted behavior is tested

### Phase 6: Live-State Preflight

Deliver:

- `pg-logstats running-queries`
- `pg_stat_activity` query
- optional `pg_stat_statements` enrichment by query id
- current session exclusion where possible
- configured truncation/redaction for query text
- live-state verdict calculation
- `allowed_actions` and `blocked_actions`
- `active_sessions[]`
- `blocking_signals[]`
- support as a `suggest-sql` input report
- built-in running-query `suggest-sql` rules:
  - exact backend lookup by `pid`
  - blocking context via `pg_blocking_pids`

Docs:

- live-state preflight workflow
- minimum permissions
- threshold config
- verdict interpretation
- transition from `readiness` to `running-queries` to `suggest-sql`
- built-in running-query rule catalog and attribution

Acceptance criteria:

- `running-queries` works in `live_only` and `log_backed`
- `clear`, `busy`, `saturated`, and `unknown` verdicts are tested
- `suggest-sql --running-queries-file` can target a pid or query id
- running-query rules are tested for allowed, missing-identifier, and blocked
  output
- output follows the shared report contract

### Phase 7: Errors And Temp Files

Deliver errors:

- `pg-logstats errors`
- grouping by SQLSTATE when available
- fallback grouping by normalized error text
- representative evidence handles
- app, user, and database attribution when known
- report-shaped findings
- built-in error-class `suggest-sql` rule for active sessions by matching app,
  database, or user

Deliver temp files:

- `pg-logstats temp-files`
- parse PostgreSQL temporary-file log events
- group by query family or nearby statement when available
- rank by temporary-file count and total bytes
- track largest observed temp-file event
- representative evidence handles
- app, user, and database attribution when known
- report-shaped findings
- built-in temp-file `suggest-sql` rules:
  - `pg_stat_database` temp counters by database
  - `pg_stat_statements` temp block activity

Docs:

- error triage workflow with pgBadger prior-art references
- temp-file triage workflow with pgBadger source/report references
- required logging settings for each workflow
- unsupported-mode behavior
- built-in error-class and temp-file rule catalog and attribution

Acceptance criteria:

- `errors` and `temp-files` follow the shared report contract
- temp-file workflow requires `log_temp_files` evidence
- rankings are deterministic
- missing statement correlation is represented as a limitation, not hidden
- error-class and temp-file `suggest-sql` rules are tested for allowed,
  missing-identifier, and blocked output

### Phase 8: Agent Guidance Install

Deliver:

- `pg-logstats agent install --harness codex|claude|gemini`
- shared agent-neutral playbook content
- Codex managed block install into user-scoped `AGENTS.md`
- Claude Code skill install
- Gemini CLI command install
- configured install path overrides
- `--dry-run`
- `--status`
- idempotent managed block or file updates

Docs:

- install command usage
- default install paths
- config overrides
- installed guidance content and update behavior

Acceptance criteria:

- repeated install does not duplicate content
- status detects installed and missing guidance
- dry-run reports intended writes
- installed guidance teaches readiness-first mode handling, verdict policy,
  blocked actions, and escalation behavior

## V1 Exit Criteria

V1 is complete when:

- config loading and precedence are implemented
- the shared report contract and canonical enums are implemented
- `readiness` reports mode and limitations honestly
- `top query-families` works in `log_backed` mode
- `running-queries` works in `live_only` and `log_backed`
- `suggest-sql` has a framework/policy layer and risk-labeled output
- `suggest-sql` supports external rule commands
- query-family, running-query, error-class, and temp-file built-in
  `suggest-sql` rules ship with their owning workflows
- `errors` and `temp-files` use the same report contract
- README and installed harness guidance reflect the shipped behavior
- every shipped built-in workflow and built-in `suggest-sql` rule has recorded
  attribution to PostgreSQL docs, pgBadger prior art, or another credible
  PostgreSQL operational source

## Open Questions

- What exact `log_line_prefix` token patterns should V1 accept for
  stderr-style correlation beyond the current parser-supported defaults?
- What is the minimum useful shared playbook content for Codex, Claude Code, and
  Gemini CLI without creating three divergent variants?
- What warning level should `pg-logstats` apply to suspicious SQL returned by
  trusted external rule commands?
- Should `slow-queries diff` be hidden, documented as experimental, or adapted
  into the V1 report contract later?
- Should live database tests use a real Postgres service, captured row fixtures,
  or a trait-backed probe abstraction with mocked responses?
- Should `agent install` stay last, or move earlier once `readiness` and
  `suggest-sql` are stable enough for guidance?
