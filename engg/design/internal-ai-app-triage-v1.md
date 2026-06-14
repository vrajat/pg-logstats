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
- Define a clear inspection contract between PostgreSQL, `pg-logstats`, and the
  consuming agent.
- Make degraded operation explicit and machine-readable.
- Ship a narrow CLI surface that is sufficient for first-pass triage.
- Make behavior configurable from V1 while keeping guidance rules built into
  `pg-logstats`.
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
and the workflow documentation. Built-in reason codes and built-in next-action
rules should also be traceable to the same rationale.

Initial V1 implementation reference map:

| Command or Rule Family | Why It Is In V1 | Implementation References |
| --- | --- | --- |
| `inspect` | PostgreSQL evidence quality determines whether log-backed or live-only claims are honest. | PostgreSQL logging configuration docs; PostgreSQL `pg_stat_statements` docs; PostgreSQL predefined roles docs; pgBadger PostgreSQL configuration section for required logging and `log_line_prefix` patterns; pgBadger source header comments for supported logging prerequisites. |
| `running-queries` | Active sessions, waits, query text visibility, and aggregate statement stats are standard first-pass live triage inputs. | PostgreSQL `pg_stat_activity` docs for active sessions, application names, waits, backend state, query text, and `query_id`; PostgreSQL `pg_stat_statements` docs for aggregate query stats; PostgreSQL predefined roles docs for cluster-wide stats visibility. |
| `top query-families` | Ranking slow, frequent, and time-consuming query families is a long-standing PostgreSQL log-analysis workflow. | pgBadger feature docs for slowest queries, time-consuming queries, frequent queries, and users/applications involved in top queries; pgBadger source functions `print_time_consuming`, `print_slowest_individual_queries`, and `print_slowest_queries`; PostgreSQL `pg_stat_statements` docs for query-family aggregate fields and `queryid`. |
| `errors` | Grouped PostgreSQL errors and error classes are common incident-triage signals. | pgBadger feature docs for most frequent errors, error events, and error class distribution; pgBadger source functions `print_error_code`, `show_error_as_html`, and `show_pgb_error_as_html`; PostgreSQL error reporting and logging docs. |
| `temp-files` | Temporary file volume is a PostgreSQL-specific pressure signal, often tied to sorts, hashes, and `work_mem`-sensitive plans. | pgBadger feature docs for queries generating the most temporary files, queries generating the largest temporary files, and temporary-file statistics; pgBadger source functions `print_tempfile_report` and `print_temporary_file`; PostgreSQL `log_temp_files` docs; PostgreSQL `pg_stat_statements` docs for `temp_blks_read` and `temp_blks_written`. |
| query-family SQL actions | Follow-up SQL can bridge a query-family finding into standard PostgreSQL stats and activity views. | pgBadger top-query reports as prior art for preserving query text, user, database, application, and queryid in findings; PostgreSQL `pg_stat_statements` docs for exact `queryid` lookups; PostgreSQL `pg_stat_activity` docs for active-session lookups by database, user, application name, pid, and query id. |
| running-query SQL actions | Live follow-up can inspect one backend or its blocking context without broad scans. | PostgreSQL `pg_stat_activity` docs for backend state, wait event fields, and query text; PostgreSQL docs for `pg_blocking_pids`; PostgreSQL predefined roles docs for visibility limits. |
| error-class SQL actions | Error findings can safely lead to current activity checks by the same app, database, or user, but not historical SQLSTATE queries from catalogs. | pgBadger error reports as prior art for grouping error message, SQLSTATE, database, user, application, and sample details; PostgreSQL error reporting docs; PostgreSQL `pg_stat_activity` docs for bounded live activity checks. |
| temp-file SQL actions | Temp-file findings can safely lead to stats-view checks for database temp counters or query-family temp block counters. | pgBadger temporary-file reports as prior art for ranking by count and size; PostgreSQL `log_temp_files` docs; PostgreSQL `pg_stat_database` docs for database temp counters; PostgreSQL `pg_stat_statements` docs for temp block counters. |

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
- report-level SQL next actions

### `live_only`

Historical logs are unavailable or insufficient, but low-impact live-state
inspection is still possible.

This mode allows:

- `inspect`
- `running-queries`
- limited report-level SQL next actions for live-state or aggregate follow-up

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

- `inspect`

This mode should produce an explicit explanation of what is missing.

## Investigation Guidance

V1 reports should model triage as a directed acyclic investigation graph with
agent judgement at branch points.

`pg-logstats` owns:

- the set of valid next actions from each report
- safety and mode constraints for those actions
- stable action identifiers and machine-readable reasons
- enough context for an agent to choose the next branch

The consuming agent owns:

- judging which optional branch fits the incident
- choosing one action when several are plausible
- stopping or escalating when the evidence is insufficient
- recording why it selected a branch when that judgement is not deterministic

The framework name is **Investigation Guidance**. The report field is
`next_actions[]`. There is no `pg-logstats next-action` command in V1. Every
workflow can emit its own next actions as part of its `PgTriageReport`.

### Next Action Shape

Every `next_actions[]` item should be compact, stable, and safe to show to both
humans and agents.

Required fields:

- `action_id`: stable identifier for this edge in the investigation graph
- `kind`: one of the canonical action kinds
- `label`: short human-readable title
- `status`: whether the action is allowed, blocked, or omitted
- `priority`: whether the action is required, recommended, or optional
- `judgement_required`: true when the agent must choose based on incident
  context that `pg-logstats` cannot infer
- `reason`: concise explanation for why this action is available or blocked

Optional fields:

- `target`: selected finding, session, query family, SQLSTATE, or other report
  object this action applies to
- `workflow`: destination `pg-logstats` workflow when `kind =
  "run_pg_logstats"`
- `command`: argv for a follow-up `pg-logstats` command
- `sql_preview`: optional SQL preview when `kind = "run_sql"`; the agent must
  not execute this text directly
- `parameters[]`: parameter names and sources needed by a SQL action
- `risk`: risk label for SQL or otherwise sensitive actions
- `action_class`: policy class used for allow/block decisions
- `required_identifiers[]`: identifiers required by the action
- `requires[]`: machine-readable preconditions
- `produces[]`: expected report/workflow artifacts

Canonical action kinds:

- `run_pg_logstats`
- `run_sql`
- `install_agent_guidance`
- `collect_logs`
- `escalate`
- `stop`

`run_sql` is still a `pg-logstats`-owned action. The agent selects the action by
`action_id`; `pg-logstats` validates policy again, binds parameters from the
source report, executes the built-in SQL through its configured DSN, and emits a
new report. The SQL preview exists for review and audit, not as an instruction
for the agent to run SQL outside `pg-logstats`.

Canonical priorities:

- `required`
- `recommended`
- `optional`

Canonical statuses:

- `allowed`
- `blocked_by_mode`
- `blocked_by_verdict`
- `blocked_by_policy`
- `blocked_by_config`
- `omitted_not_enough_context`
- `omitted_unsupported_target`

Illustrative next action:

```json
{
  "action_id": "inspect.log_backed.top_query_families",
  "kind": "run_pg_logstats",
  "workflow": "top_query_families",
  "label": "Rank query families from the available log window",
  "status": "allowed",
  "priority": "recommended",
  "judgement_required": true,
  "reason": "Log-backed mode is available. Use this when the incident appears query-latency related.",
  "command": {
    "argv": [
      "pg-logstats",
      "top",
      "query-families",
      "--output-format",
      "json"
    ]
  },
  "requires": ["operating_mode:log_backed"],
  "produces": ["workflow:top_query_families"]
}
```

### Guidance Rules

Guidance rules are the rule engine that creates `next_actions[]`.

They take:

- current workflow
- current report payload
- operating mode
- verdict and verdict reasons
- limitations
- allowed and blocked action classes
- available source artifacts in the workspace or session

They produce:

- zero or more action candidates
- omitted or blocked actions when useful for explaining constraints
- omitted or blocked actions when useful for explaining constraints

Guidance rules should be deterministic for the same input report. The
non-deterministic part of the triage remains outside `pg-logstats`: the agent
chooses between optional or recommended actions using incident context,
application knowledge, and user instructions.

Examples:

- `inspect` in `log_backed` mode may offer `top query-families`, `errors`,
  `temp-files`, and `running-queries` as possible next actions.
- `inspect` in `live_only` mode should offer `running-queries` and omit
  log-backed workflows with machine-readable blocked reasons.
- `top query-families` may offer SQL actions for a selected finding, or may
  offer `running-queries` when the current incident appears ongoing.
- any report may offer `install_agent_guidance` when agent inspect checks fail.
- any report may offer `escalate` or `stop` when evidence is insufficient or
  the verdict blocks safe agent-driven progress.

### Sessions And Replay

V1 should treat reports as the primary artifacts and breadcrumbs as a derived
debugging aid.

When report persistence is enabled, reports should be written under the workspace:

- `<workspace>/reports/<timestamp>-<workflow>.json`

Each stored report should include enough metadata to reconstruct the
investigation graph:

- `report_id`
- `parent_report_id`
- `selected_action_id`
- `created_at`

This makes it possible to replay an investigation by following the stored
reports and selected actions. The session trail should not replace the report
contract; it exists for debugging, audit, and introspection.

## Inspection Contracts

### Database Inspection

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
`log_backed` inspection until the implementation explicitly supports parsing
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

### Agent Inspection

The agent environment is ready when:

- `pg-logstats` has installed the harness-specific guidance bundle for the
  selected agent surface
- that installed guidance tells the agent exactly how to run `inspect` first,
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

1. run `pg-logstats inspect --output-format json`
2. inspect `operating_mode`
3. choose from `next_actions[]` instead of inventing unsupported branches
4. respect `verdict`, `allowed_actions`, `blocked_actions`, and next-action
   status fields
5. stop and escalate when the report says evidence is insufficient or the
   database is saturated

## Configuration Model

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
- enabled and disabled built-in guidance rule IDs
- maximum risk emitted for SQL next actions
- whether omitted or blocked next actions are emitted
- per-rule limits
- user-level install targets for Codex, Claude Code, and Gemini CLI guidance
- query text truncation or redaction limits

Guidance rules should be built into `pg-logstats` in V1. Configuration can
enable, disable, and limit those rules, but it should not define new rule
behavior.

Example config:

```toml
[database]
dsn = "postgres://app_observer@db.example.com:5432/internal_tools"
connect_timeout_ms = 3000

[running_queries.thresholds]
long_running_query_ms = 120000
waiting_session_count_threshold = 2
idle_in_transaction_count_threshold = 2

[guidance]
max_risk = "bounded"
show_omitted_actions = true
disabled_rules = [
  "query_family.pg_stat_statements.by_query_pattern"
]

[guidance.rules.query_family.pg_stat_activity.by_dimensions]
limit = 50

[agent_install.codex]
agents_md_path = "/Users/example/AGENTS.md"

[agent_install.claude]
skill_dir = "/Users/example/.claude/skills/pg-logstats-triage"

[agent_install.gemini]
commands_dir = "/Users/example/.gemini/commands"
```

Other V1 behavior should also be config-driven:

- verdict thresholds for live-state checks
- agent harness install paths
- query text truncation or redaction limits
- built-in rule enablement and per-rule limits

V1 should keep these identifiers fixed by the implementation:

- operating mode names
- top-level report schema
- inspect check names
- finding kinds emitted by built-in workflows
- guidance rule loading

## V1 Contract Tables

These tables are normative for implementation. Later narrative and examples
should be read through these tables when there is any ambiguity.

### Evidence Source Support

| Evidence source | V1 inspection status | Notes |
| --- | --- | --- |
| Local supported stderr-style logs | Can satisfy `log_backed` | Requires a parser-recognized format, reachable files, statement text, duration evidence, and session or process identity for correlation. |
| AWS RDS / CloudWatch PostgreSQL logs in a parser-supported text shape | Can satisfy `log_backed` | Requires the same statement, duration, and identity evidence as local stderr-style logs. |
| `csvlog` | Documented target, not V1-ready unless parser support is implemented in the same phase | `inspect` must report `unsupported_log_format` rather than count this as `log_backed` merely because `log_destination` includes `csvlog`. |
| `jsonlog` | Documented target, not V1-ready unless parser support is implemented in the same phase | `inspect` must report `unsupported_log_format` rather than count this as `log_backed` merely because `log_destination` includes `jsonlog`. |
| `pg_stat_activity` | Required for `live_only` and `running-queries` | Must be queryable with enough visibility to inspect cluster-wide activity. |
| `pg_stat_statements` | Required for `live_only` and `running-queries` in V1 | Must be loaded, installed in the target database, and queryable. Aggregate history from this view is non-log history, not log-backed event history. |

### Database Connection Discovery

Commands that need live PostgreSQL checks must resolve a connection target in
this order:

1. `--dsn <postgres-url>`
2. `PG_LOGSTATS_DATABASE_URL`
3. `[database].dsn` in the resolved config file

If no connection target is available:

- `inspect` should still perform any requested static or log-source checks
  and report live checks as `skipped` with reason
  `database_connection_not_configured`
- `inspect` must not choose `live_only`
- `running-queries` must fail with a structured error that identifies
  `database_connection_not_configured`
- `run-action` must fail for `run_sql` actions with a structured error that
  identifies `database_connection_not_configured`

The exact PostgreSQL client crate is an implementation choice, but V1 must
document accepted DSN forms, SSL behavior, timeout defaults, and how connection
errors appear in JSON output before `inspect` is considered complete.

### Command Inputs

| Command | Required inputs | Optional inputs | Supported modes |
| --- | --- | --- | --- |
| `pg-logstats agent install --harness codex\|claude\|gemini` | harness | `--config` and configured install path overrides | local tool operation, independent of database mode |
| `pg-logstats inspect` | none | `--dsn`, log input args, `--config` | reports `log_backed`, `live_only`, or `unready` |
| `pg-logstats running-queries` | resolvable database connection | `--config`, threshold overrides if exposed as CLI flags | `live_only`, `log_backed` |
| `pg-logstats top query-families` | supported log input and bounded window | `--config`, `--limit`, source-specific input flags | `log_backed` |
| `pg-logstats errors` | supported log input and bounded window | `--config`, `--limit`, source-specific input flags | `log_backed` |
| `pg-logstats temp-files` | supported log input and bounded window with temp-file evidence | `--config`, `--limit`, source-specific input flags | `log_backed` |
| `pg-logstats run-action --report <path> --action-id <id>` | source report and action id | `--config`, `--dsn` when action requires SQL | mode inherited from source report and action policy |

There is no separate `next-action` command in V1. Follow-up guidance is emitted
inside each report as `next_actions[]`.

`run-action` is an executor, not a recommender. The agent still chooses an
allowed action from `next_actions[]`; `pg-logstats` executes that selected
action, records it in the session when one is active, and emits the next
`PgTriageReport`.

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
| `analysis_window` | required for log-window workflows | Omit for `inspect`, `agent install`, and point-in-time live snapshots. |
| `source_summary` | required for evidence-producing workflows | Summarizes log source or live views consulted. |
| `next_actions[]` | yes | Investigation Guidance actions available after this report. Empty only when the report tells the agent to stop or no safe action is available. |
| `payload` | yes | Command-specific object. |

Command-specific payload keys:

| Workflow | Payload key |
| --- | --- |
| `agent_install` | `agent_install` |
| `inspect` | `inspect` |
| `running_queries` | `running_queries` |
| `top_query_families` | `findings` |
| `errors` | `findings` |
| `temp_files` | `findings` |
| `sql_action` | `sql_action` |

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
- `inspect`
- `running_queries`
- `top_query_families`
- `errors`
- `temp_files`
- `sql_action`

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

Next-action statuses:

- `allowed`
- `blocked_by_mode`
- `blocked_by_verdict`
- `blocked_by_config`
- `blocked_by_policy`
- `omitted_not_enough_context`
- `omitted_unsupported_target`

Next-action kinds:

- `run_pg_logstats`
- `run_sql`
- `install_agent_guidance`
- `collect_logs`
- `escalate`
- `stop`

Next-action priorities:

- `required`
- `recommended`
- `optional`

### Verdict Policy Matrix

| Verdict | Allowed action classes | Blocked action classes | Agent instruction |
| --- | --- | --- | --- |
| `clear` | `system_catalog_reads`, `stats_view_reads`, `bounded_activity_queries`, `text_pattern_stats_search`, `explain_without_analyze` | `large_unbounded_selects`, `explain_analyze`, `write_or_admin_action` | Continue with bounded diagnostic reads. |
| `busy` | `system_catalog_reads`, `stats_view_reads`, `bounded_activity_queries` | `text_pattern_stats_search`, `explain_without_analyze`, `large_unbounded_selects`, `explain_analyze`, `write_or_admin_action` | Keep follow-up narrow and low-impact. |
| `saturated` | none by default | all action classes | Stop adding investigative database load and escalate with the report. |
| `unknown` | omitted | omitted | Do not infer safety; escalate or ask for better evidence. |

Config may only make this matrix more restrictive. It must not allow an action
class that the verdict blocks.

### Built-In Guidance Rule Registry Contract

Built-in guidance rules must be declared in a central registry with:

- stable `rule_id`
- emitted `action_id`
- emitted action `kind`
- supported target `workflow`
- supported target `kind`
- required identifiers
- emitted `risk` and `action_class` when the action is SQL or otherwise
  policy-sensitive
- command, SQL template, or action generator name
- attribution note or reference back to the workflow attribution map

V1 built-in guidance rule IDs should use these prefixes:

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

[guidance]
max_risk = "bounded"
show_omitted_actions = true
disabled_rules = []

[guidance.rules.<rule_id>]
enabled = true
limit = 20

[agent_install.codex]
agents_md_path = "/Users/example/AGENTS.md"
playbook_dir = "/Users/example/.config/pg-logstats/agents"
```

### Inspection Checks By Mode

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

### `pg-logstats inspect`

Purpose:

- detect operating mode before deeper investigation
- report database evidence and `pg-logstats` capabilities

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
- `database_inspect`
- `agent_inspect`
- `required_checks`
- `failed_checks`
- limitations
- top-level `next_actions[]`

Example fields:

- `database_inspect.mode_candidate`
- `database_inspect.checks.log_duration`
- `database_inspect.checks.pg_stat_statements_extension`
- `agent_inspect.codex.installed`
- `agent_inspect.claude.installed`
- `agent_inspect.gemini.installed`

Illustrative JSON:

```json
{
  "schema_version": 1,
  "workflow": "inspect",
  "operating_mode": "live_only",
  "database_inspect": {
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
  "agent_inspect": {
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
  "next_actions": [
    {
      "action_id": "inspect.live_only.running_queries",
      "kind": "run_pg_logstats",
      "workflow": "running_queries",
      "label": "Inspect current PostgreSQL activity",
      "status": "allowed",
      "priority": "recommended",
      "judgement_required": false,
      "reason": "Live-only mode is available, so current activity can be inspected safely.",
      "command": {
        "argv": ["pg-logstats", "running-queries", "--output-format", "json"]
      },
      "requires": ["operating_mode:live_only"],
      "produces": ["workflow:running_queries"]
    }
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
- top-level `next_actions[]` for plausible follow-up branches

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
- top-level `next_actions[]` when follow-up is safe

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
- top-level `next_actions[]` when follow-up is safe

Supported modes:

- `log_backed`

### `pg-logstats run-action`

Purpose:

- execute one allowed action previously emitted in a `PgTriageReport`
- keep SQL execution, result capture, policy checks, and session breadcrumbs
  inside `pg-logstats`
- emit a new report that can continue the investigation DAG

Inputs:

- `--report <path>`: source report containing the selected action
- `--action-id <id>`: action to execute
- `--dsn <postgres-url>` or configured database connection when the action is
  `kind = "run_sql"`

Execution rules:

- load the source report and validate `schema_version`
- find exactly one action with the requested `action_id`
- require `status = "allowed"`
- re-apply mode, verdict, action-class, and risk policy before execution
- for `run_pg_logstats`, run the referenced command or dispatch the equivalent
  workflow internally
- for `run_sql`, bind parameters from the source report and execute only the
  built-in SQL action identified by `action_id`
- write the resulting report to the active session when a session is active
- include `parent_report_id` and `selected_action_id` in the resulting report

Output schema for SQL actions:

- `workflow = "sql_action"`
- source report identifier
- selected `action_id`
- SQL action metadata: `risk`, `action_class`, and `required_identifiers[]`
- bounded result rows or typed summary fields
- row count and truncation indicators
- top-level `next_actions[]`

`run-action` must not accept arbitrary SQL from the agent. SQL execution is
limited to built-in SQL actions emitted by prior reports.

### Report-Level Guidance Actions

Purpose:

- emit safe, machine-readable next actions as part of every `PgTriageReport`
- make the triage DAG explicit without adding a separate next-action command
- let agents choose between optional branches using incident-specific judgement

Sources:

- the current report payload
- prior reports from the active session when available
- built-in guidance rule registry
- verdict, operating mode, limitations, and action policy

V1 guidance lifecycle:

`pg-logstats` should use a simple built-in rule lifecycle. Rules are registered
in code as DAG edge producers. Each rule declares the source workflow and source
target shape it can inspect, then emits zero or more next-action candidates.

1. Discover rule sources:
   - load the built-in guidance registry
   - apply config for disabled built-in rules
2. Build guidance context:
   - current report metadata
   - current workflow
   - current payload
   - `operating_mode`
   - `verdict`
   - `limitations`
   - `allowed_actions`
   - `blocked_actions`
   - session report index when available
3. Evaluate and generate candidates:
   - built-in rules evaluate applicability and generate action candidates
4. Normalize candidates:
   - validate required fields
   - attach `rule_source`
5. Apply pg-logstats policy:
   - enforce mode support
   - enforce `max_risk` for SQL actions
   - enforce `blocked_actions`
   - classify candidates by `status`
   - sort actions by priority, status, specificity, and stable action ID

This lifecycle keeps `pg-logstats` responsible for graph edges and safety
policy. It deliberately does not decide which optional branch the agent must
take.

V1 guidance algorithm:

1. Build a guidance context from the just-created report.
2. Include session context when a session directory is active.
3. Generate action candidates from built-in guidance rules.
4. Derive common identifiers for SQL action generation when needed:
   - `query_id` or `queryid`
   - `application_name`
   - `database`
   - `user`
   - `pid`
   - SQLSTATE or normalized error text
5. Mark candidates as blocked when their `action_class` appears in
   `blocked_actions`, when their mode is unsupported, or when their risk maps to
   a blocked action.
6. Keep blocked candidates only when useful for explaining why a branch is
   unavailable.
7. Emit the surviving actions as top-level `next_actions[]`.

Validation checks:

- every emitted action must match the next-action schema
- every `action_id` must be stable for the same rule and target
- `run_pg_logstats` actions must reference a known workflow
- `run_sql` actions must include a `run-action` command, not a command that
  invokes a SQL client directly
- SQL actions must use only read-only `SELECT` statements
- SQL template values must be SQL-literal escaped
- SQL parameters must declare their source in the source report
- any SQL pattern match must use an explicit `LIMIT`
- generated SQL must include enough predicates to match the declared risk label
- SQL templates with missing required identifiers must be omitted instead of
  emitted with empty predicates

Built-in SQL actions must never generate:

- DDL
- DML
- `VACUUM`
- `ANALYZE`
- `EXPLAIN ANALYZE`
- unbounded user-table queries
- queries that require table names inferred from arbitrary SQL parsing

This "never generate" list applies to built-in SQL action rules.

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
  - valid diagnostic SQL exists, but V1 should not emit it as an allowed
    agent-selectable `run_sql` action
  - examples include `EXPLAIN`, `EXPLAIN ANALYZE`, lock termination, index
    creation, or table-specific inspection

Risk is assigned from the SQL source and predicate shape, not from the severity
of the finding. A severe finding can still have only `safe` follow-up SQL.

Config can reduce the emitted risk ceiling. For example, `max_risk = "safe"`
means `bounded`, `expensive`, and `requires_human_approval` candidates move to
`next_actions[]` with an omitted or blocked status.

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
  - never emitted as `status = "allowed"`; record only as omitted or blocked
    action metadata

Action output schema:

- top-level `next_actions[]`
- optional top-level or payload-level `rule_sources[]`

Per-item fields:

- `label`
- `action_id`
- `kind`
- `priority`
- `judgement_required`
- `rule_source`
- `risk`
- `action_class`
- `status`
- `command`
- `sql_preview`
- `parameters[]`
- `reason`
- `required_identifiers[]`

`rule_source` values:

- `built_in`

`status` values:

- `allowed`
- `blocked_by_mode`
- `blocked_by_verdict`
- `blocked_by_policy`
- `blocked_by_config`
- `omitted_not_enough_context`
- `omitted_unsupported_target`

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

#### Omitted SQL actions

When V1 intentionally does not allow runnable SQL, it should record the omitted
or blocked action:

```json
{
  "action_id": "query_family.explain.without_analyze",
  "kind": "run_sql",
  "label": "Explain the query plan",
  "risk": "requires_human_approval",
  "action_class": "explain_without_analyze",
  "status": "blocked_by_verdict",
  "priority": "optional",
  "judgement_required": true,
  "reason": "The current verdict blocks plan inspection from agent-driven flow."
}
```

This lets the agent produce a better escalation report without running unsafe or
underspecified SQL.

Illustrative output:

```json
{
  "schema_version": 1,
  "workflow": "top_query_families",
  "operating_mode": "log_backed",
  "verdict": "busy",
  "payload": {
    "findings": [
      {
        "finding_id": "query_family:app=invoice-helper:sql=...",
        "kind": "query_family",
        "rank": 1
      }
    ]
  },
  "next_actions": [
    {
      "action_id": "query_family.pg_stat_statements.by_queryid",
      "kind": "run_sql",
      "label": "Inspect statement statistics for the exact query family",
      "rule_id": "query_family.pg_stat_statements.by_queryid",
      "rule_source": "built_in",
      "risk": "safe",
      "action_class": "stats_view_reads",
      "status": "allowed",
      "priority": "recommended",
      "judgement_required": false,
      "command": {
        "argv": [
          "pg-logstats",
          "run-action",
          "--report",
          "<current-report>",
          "--action-id",
          "query_family.pg_stat_statements.by_queryid"
        ]
      },
      "sql_preview": "SELECT queryid, calls, total_exec_time, mean_exec_time, min_exec_time, max_exec_time, rows, shared_blks_hit, shared_blks_read, temp_blks_read, temp_blks_written, query FROM pg_stat_statements WHERE queryid = $1;",
      "parameters": [
        {
          "name": "queryid",
          "source": "target.queryid"
        }
      ],
      "reason": "The finding includes queryid, so this is an exact stats-view lookup.",
      "required_identifiers": ["queryid"],
      "target": {
        "finding_id": "query_family:app=invoice-helper:sql=...",
        "queryid": "918273645"
      },
      "produces": ["workflow:sql_action"]
    },
    {
      "action_id": "query_family.pg_stat_activity.by_dimensions",
      "kind": "run_sql",
      "label": "Find current active sessions for the same app and database",
      "rule_id": "query_family.pg_stat_activity.by_dimensions",
      "rule_source": "built_in",
      "risk": "safe",
      "action_class": "bounded_activity_queries",
      "status": "allowed",
      "priority": "optional",
      "judgement_required": true,
      "command": {
        "argv": [
          "pg-logstats",
          "run-action",
          "--report",
          "<current-report>",
          "--action-id",
          "query_family.pg_stat_activity.by_dimensions"
        ]
      },
      "sql_preview": "SELECT pid, usename, datname, application_name, state, wait_event_type, wait_event, query_start, query_id, query FROM pg_stat_activity WHERE datname = $1 AND usename = $2 AND application_name = $3 AND state <> 'idle' ORDER BY query_start DESC NULLS LAST LIMIT 20;",
      "parameters": [
        {
          "name": "database",
          "source": "target.database"
        },
        {
          "name": "user",
          "source": "target.user"
        },
        {
          "name": "application_name",
          "source": "target.application_name"
        }
      ],
      "reason": "The query is bounded to the app, database, and user dimensions from the finding.",
      "required_identifiers": ["database", "user", "application_name"],
      "target": {
        "finding_id": "query_family:app=invoice-helper:sql=...",
        "database": "internal_tools",
        "user": "app_user",
        "application_name": "invoice-helper"
      },
      "produces": ["workflow:sql_action"]
    },
    {
      "action_id": "query_family.explain.without_analyze",
      "kind": "run_sql",
      "label": "Explain the query plan",
      "rule_id": "query_family.explain.without_analyze",
      "rule_source": "built_in",
      "risk": "requires_human_approval",
      "action_class": "explain_without_analyze",
      "status": "blocked_by_verdict",
      "priority": "optional",
      "judgement_required": true,
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
  "next_actions": [
    {
      "action_id": "query_family.pg_stat_activity.by_dimensions",
      "kind": "run_sql",
      "label": "Find current active sessions for the same app and database",
      "status": "allowed",
      "priority": "optional",
      "judgement_required": true,
      "command": {
        "argv": [
          "pg-logstats",
          "run-action",
          "--report",
          "<current-report>",
          "--action-id",
          "query_family.pg_stat_activity.by_dimensions"
        ]
      },
      "produces": ["workflow:sql_action"],
      "reason": "Use this branch when the historical query-family finding appears related to current database pressure."
    }
  ],
  "payload": {
    "findings": []
  }
}
```

### Degraded Output Requirements

`pg-logstats inspect` is the authoritative mode and capability report.

It should:

- state the active operating mode explicitly
- enumerate the important limitations of that mode
- emit `next_actions[]` for supported follow-up branches

Other commands do not need to repeat the full degraded explanation every time.
They should carry only enough mode metadata to prevent misuse when the current
mode changes what the command can honestly claim.

Example:

```json
{
  "schema_version": 1,
  "workflow": "inspect",
  "operating_mode": "live_only",
  "limitations": [
    "historical_log_triage_unavailable",
    "query_family_runtime_ranking_unavailable"
  ],
  "next_actions": [
    {
      "action_id": "inspect.live_only.running_queries",
      "kind": "run_pg_logstats",
      "workflow": "running_queries",
      "label": "Inspect current PostgreSQL activity",
      "status": "allowed",
      "priority": "recommended",
      "judgement_required": false,
      "reason": "Live-only mode is available, so current activity can be inspected safely.",
      "command": {
        "argv": ["pg-logstats", "running-queries", "--output-format", "json"]
      }
    }
  ],
  "payload": {
    "inspect": {}
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
`top query-families`, `slow-queries diff`, and basic follow-up SQL support, so
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
  action classes, next-action kinds, next-action priorities, next-action
  statuses, finding kinds, and check statuses
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

### Phase 2: Inspect And Mode Detection

Deliver:

- `pg-logstats inspect`
- database connection discovery from `--dsn`, `PG_LOGSTATS_DATABASE_URL`, or
  `[database].dsn`
- lightweight PostgreSQL probe runner
- log-source reachability checks
- parser-supported evidence checks for `log_backed`
- `live_only` checks for `pg_stat_activity`, `pg_stat_statements`,
  `compute_query_id`, and monitoring-role visibility
- agent guidance artifact detection
- operating mode detection: `log_backed`, `live_only`, `unready`
- machine-readable limitations and `next_actions[]`

Docs:

- inspect-first workflow
- supported and unsupported evidence sources
- database permissions and connection assumptions
- examples for `log_backed`, `live_only`, and `unready`

Acceptance criteria:

- missing database connection records live checks as skipped, not passed
- `csvlog` and `jsonlog` do not satisfy `log_backed` until parser support exists
- inspect can report useful log-backed status without live DB access when log
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

### Phase 4: Investigation Guidance Framework

Deliver:

- report-level `next_actions[]`
- next-action schema with `action_id`, `kind`, `label`, `status`, `priority`,
  `judgement_required`, `reason`, `target`, `command`, `sql_preview`,
  `parameters[]`, `requires[]`, and `produces[]`
- `pg-logstats run-action --report <path> --action-id <id>`
- `sql_action` report payload for SQL action execution results
- canonical next-action kinds, priorities, and statuses
- guidance rule registry abstraction
- built-in rule source interface
- guidance context JSON for rule evaluation
- candidate normalization
- mode, verdict, action-class, and risk policy filtering
- config controls for rule enablement, risk ceiling for SQL actions, omitted
  actions, and per-rule limits
- session report metadata needed to replay an investigation DAG

Docs:

- Investigation Guidance overview
- next-action schema
- run-action executor behavior
- SQL action result report shape
- guidance rule lifecycle
- session and replay behavior
- risk labels
- action classes
- policy matrix
- examples for allowed, omitted, and blocked built-in actions

Acceptance criteria:

- there is no separate `next-action` command
- every machine-readable report includes `next_actions[]`
- `run-action` can execute an allowed action from a source report
- `run-action` rejects blocked or unknown actions with structured errors
- SQL action execution is owned by `pg-logstats`, not the agent harness
- next actions are never unlabeled
- blocked classes are machine-readable
- config can only make policy stricter
- no real built-in SQL action needs to ship in this phase beyond test fixtures or
  a minimal internal fixture action

### Phase 5: Initial Built-In SQL Actions

Deliver query-family SQL actions:

- exact `pg_stat_statements` lookup by `queryid`
- active sessions by app, database, and user dimensions
- bounded `pg_stat_statements` text search fallback when `queryid` is missing
- `run-action` execution for the query-family SQL actions above
- `sql_action` reports containing bounded rows, row counts, truncation flags,
  source report id, and selected `action_id`

Do not implement running-query, error-class, or temp-file SQL action rules in
this phase unless their source report workflows already exist. Those rule
families should ship with the workflows that create their target reports.

Docs:

- built-in query-family rule catalog with stable rule IDs
- required identifiers per rule
- risk and action class per rule
- attribution for the query-family rule family

Acceptance criteria:

- every query-family built-in SQL action rule has tests for allowed output
- every query-family built-in SQL action rule has tests for missing identifiers
- every executable query-family SQL action has tests for parameter binding and
  `sql_action` report output
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
- `next_actions[]` for plausible follow-up branches
- built-in running-query SQL action rules:
  - exact backend lookup by `pid`
  - blocking context via `pg_blocking_pids`

Docs:

- live-state preflight workflow
- minimum permissions
- threshold config
- verdict interpretation
- transition from `inspect` to `running-queries` through `next_actions[]`
- built-in running-query rule catalog and attribution

Acceptance criteria:

- `running-queries` works in `live_only` and `log_backed`
- `clear`, `busy`, `saturated`, and `unknown` verdicts are tested
- running-query SQL actions can target a pid or query id
- running-query SQL action rules are tested for allowed, missing-identifier, and
  blocked output
- output follows the shared report contract

### Phase 7: Errors And Temp Files

Deliver errors:

- `pg-logstats errors`
- grouping by SQLSTATE when available
- fallback grouping by normalized error text
- representative evidence handles
- app, user, and database attribution when known
- report-shaped findings
- built-in error-class SQL action rule for active sessions by matching app,
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
- built-in temp-file SQL action rules:
  - `pg_stat_database` temp counters by database
  - `pg_stat_statements` temp block activity

Docs:

- error triage workflow with pgBadger prior-art references
- temp-file triage workflow with pgBadger source/report references
- required logging settings for each workflow
- unsupported-mode behavior
- built-in error-class and temp-file SQL action rule catalog and attribution

Acceptance criteria:

- `errors` and `temp-files` follow the shared report contract
- temp-file workflow requires `log_temp_files` evidence
- rankings are deterministic
- missing statement correlation is represented as a limitation, not hidden
- error-class and temp-file SQL action rules are tested for allowed,
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
- installed guidance teaches inspect-first mode handling, verdict policy,
  blocked actions, and escalation behavior

## V1 Exit Criteria

V1 is complete when:

- config loading and precedence are implemented
- the shared report contract and canonical enums are implemented
- `inspect` reports mode and limitations honestly
- `top query-families` works in `log_backed` mode
- `running-queries` works in `live_only` and `log_backed`
- Investigation Guidance has a framework/policy layer and report-level
  `next_actions[]`
- query-family, running-query, error-class, and temp-file built-in
  SQL action rules ship with their owning workflows
- `errors` and `temp-files` use the same report contract
- README and installed harness guidance reflect the shipped behavior
- every shipped built-in workflow and built-in SQL action rule has recorded
  attribution to PostgreSQL docs, pgBadger prior art, or another credible
  PostgreSQL operational source

## Open Questions

- What exact `log_line_prefix` token patterns should V1 accept for
  stderr-style correlation beyond the current parser-supported defaults?
- What is the minimum useful shared playbook content for Codex, Claude Code, and
  Gemini CLI without creating three divergent variants?
- Should `slow-queries diff` be hidden, documented as experimental, or adapted
  into the V1 report contract later?
- Should live database tests use a real Postgres service, captured row fixtures,
  or a trait-backed probe abstraction with mocked responses?
- Should `agent install` stay last, or move earlier once `inspect` and
  Investigation Guidance are stable enough for agent playbooks?
