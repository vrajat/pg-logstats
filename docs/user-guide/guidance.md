# Investigation Guidance Framework

`pg-logstats` models the database triage process as a directed acyclic investigation graph (DAG) with developer or agent judgement at branch points. 

Instead of requiring callers to invent database-specific diagnostic commands, every triage report includes a list of safe, contextual `next_actions[]` that the caller can select from. The caller then executes the chosen action by running the corresponding CLI command (like `top query-families` or `run-sql`) while supplying audit linkage flags.

---

## The Investigation Guidance Framework

Every machine-readable triage report (JSON output) includes a top-level `next_actions[]` field. Each next action in the list has the following shape:

```json
{
  "action_id": "query_family.pg_stat_activity.by_dimensions:query_family:qf_51125b8829ab1fdf",
  "kind": "run_sql",
  "label": "Find current active sessions for the same query-family dimensions",
  "status": "allowed",
  "priority": "optional",
  "judgement_required": true,
  "reason": "The finding includes database, user, or application attribution that can bound pg_stat_activity.",
  "sql_preview": "SELECT pid, usename, datname, application_name, state, wait_event_type, wait_event, query_start, query_id, query FROM pg_stat_activity WHERE ...",
  "risk": "safe",
  "action_class": "bounded_activity_queries",
  "required_identifiers": ["database|user|application_name"]
}
```

### Action Kinds
- `top_query_families`: Rank query families.
- `run_sql`: Runs a safe, built-in diagnostic SQL query.
- `agent_install`: Installs agent skill playbooks/harnesses.
- `running_queries`: Monitor active database sessions.
- `collect_logs`: Collects additional database logs.
- `escalate`: Directs the agent to stop and notify a human operator.
- `stop`: Directs the agent that the investigation is successfully complete.

### Priorities
- `required`
- `recommended`
- `optional`

### Next Action Status
- `allowed`: The action is safe to execute in the current state.
- `blocked_by_mode`: The action requires a different operating mode (e.g. `log_backed`).
- `blocked_by_verdict`: The database safety verdict blocks this action.
- `blocked_by_config`: The action has been disabled or exceeds `max_risk` in the configuration.
- `blocked_by_policy`: The action is blocked by built-in security policies.
- `omitted_not_enough_context`: The action requires missing identifiers (e.g., missing query ID).
- `omitted_unsupported_target`: The action target is not supported.

---

## Safety Policy Matrix

To prevent diagnostic activity from adding harmful overhead to an already stressed database, actions are allowed or blocked dynamically based on the current **verdict** of the triage report.

| Verdict | Allowed Action Classes | Blocked Action Classes | Agent / Caller Instruction |
|---|---|---|---|
| `clear` | `system_catalog_reads`, `stats_view_reads`, `bounded_activity_queries`, `text_pattern_stats_search`, `explain_without_analyze` | `large_unbounded_selects`, `explain_analyze`, `write_or_admin_action` | Continue with bounded diagnostic reads. |
| `busy` | `system_catalog_reads`, `stats_view_reads`, `bounded_activity_queries` | `text_pattern_stats_search`, `explain_without_analyze`, `large_unbounded_selects`, `explain_analyze`, `write_or_admin_action` | Keep follow-up narrow and low-impact. |
| `saturated` | *None* | *All action classes* | Stop adding database load and escalate. |
| `unknown` | *None* | *All action classes* | Do not infer safety; escalate or get better evidence. |

---

## Executing Actions with Linkage Flags

Rather than running a single wrapper command, the caller executes the actual subcommand recommended by the action (using the command line provided in the `command` field of `NextAction`), and links it to the parent report using global audit flags.

### Command Usage Example
```bash
pg-logstats \
  --session-id test_sess \
  --parent-report-id 0001-top_query_families \
  --selected-action-id query_family.pg_stat_activity.by_dimensions:query_family:qf_51125b8829ab1fdf \
  run-sql
```

### Global Audit Linkage Options
- `--session-id <SESSION_ID>`: Unique identifier for the current investigation session.
- `--parent-report-id <REPORT_ID>`: The ID of the report that led to this action.
- `--selected-action-id <ACTION_ID>`: The `action_id` from the parent report's `next_actions` array.

### Behavior & Security
1. **Safety Re-evaluation**: `pg-logstats` reads the parent report, finds the requested action, and re-validates the policy matrix against the current state and parameters. If the action is blocked or unknown, execution is rejected with a structured error.
2. **Execution**: The subcommand (e.g. `run-sql`) is executed with safety checks in place.
3. **Report Output & Session Storage**: The command outputs a new triage report containing the results. If a session workspace is configured, it persists the report in the session directory under `<workspace>/sessions/<session_id>/reports/<sequence>-<workflow>.json` to record the progress of the investigation.

## Built-In Query-Family SQL Actions

Phase 5 ships two built-in query-family SQL actions with stable rule IDs:

| Rule ID | Purpose | Required identifiers | Risk | Action class | Attribution |
|---|---|---|---|---|---|
| `query_family.pg_stat_statements.by_queryid` | Exact `pg_stat_statements` lookup for the query family. | `queryid` | `safe` | `stats_view_reads` | PostgreSQL `pg_stat_statements` exact queryid lookup |
| `query_family.pg_stat_activity.by_dimensions` | Bounded `pg_stat_activity` lookup using the finding's database, user, and application attribution. | at least one of `database`, `user`, `application_name` | `safe` when `application_name` is present, otherwise `bounded` | `bounded_activity_queries` | PostgreSQL `pg_stat_activity` lookup by app, database, and user |

`run-sql` now executes only built-in SQL actions selected from a parent report. The caller can supply `--parameter NAME=VALUE`, but cannot supply raw SQL text.

## Attribution

The workflow-level attribution lives in [engg/design/internal-ai-app-triage-v1.md](https://github.com/vrajat/pg-logstats/blob/main/engg/design/internal-ai-app-triage-v1.md), especially the "Workflow Attribution And Selection" section and its initial V1 reference map.

For the query-family SQL actions in this phase, the intended prior art is:

- PostgreSQL `pg_stat_statements` documentation for exact `queryid` lookup and normalized statement identity.
- PostgreSQL `pg_stat_activity` documentation for active-session inspection by database, user, application name, wait state, and `query_id`.
- pgBadger top-query reports as report-shape prior art for carrying query text plus attribution dimensions into follow-up investigation.
