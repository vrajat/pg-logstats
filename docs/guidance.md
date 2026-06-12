# Investigation Guidance Framework

`pg-logstats` models the database triage process as a directed acyclic investigation graph (DAG) with developer or agent judgement at branch points. 

Instead of requiring callers to invent database-specific diagnostic commands, every triage report includes a list of safe, contextual `next_actions[]` that the caller can select from. The caller then executes the chosen action by running the corresponding CLI command (like `top query-families` or `run-sql`) while supplying audit linkage flags.

---

## The Investigation Guidance Framework

Every machine-readable triage report (JSON output) includes a top-level `next_actions[]` field. Each next action in the list has the following shape:

```json
{
  "action_id": "query_family.pg_stat_statements.by_query_pattern:query_family:qf_51125b8829ab1fdf",
  "kind": "run_sql",
  "label": "Lookup query stats in pg_stat_statements",
  "status": "allowed",
  "priority": "recommended",
  "judgement_required": true,
  "reason": "Query normalized text is available. Search pg_stat_statements for matching stats.",
  "sql_preview": "SELECT queryid, calls, total_exec_time, ... FROM pg_stat_statements WHERE query ILIKE ...",
  "risk": "bounded",
  "action_class": "text_pattern_stats_search",
  "required_identifiers": ["normalized_sql"]
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
  --selected-action-id query_family.pg_stat_statements.by_query_pattern:query_family:qf_51125b8829ab1fdf \
  run-sql --sql "SELECT 1;"
```

### Global Audit Linkage Options
- `--session-id <SESSION_ID>`: Unique identifier for the current investigation session.
- `--parent-report-id <REPORT_ID>`: The ID of the report that led to this action.
- `--selected-action-id <ACTION_ID>`: The `action_id` from the parent report's `next_actions` array.

### Behavior & Security
1. **Safety Re-evaluation**: `pg-logstats` reads the parent report, finds the requested action, and re-validates the policy matrix against the current state and parameters. If the action is blocked or unknown, execution is rejected with a structured error.
2. **Execution**: The subcommand (e.g. `run-sql`) is executed with safety checks in place.
3. **Report Output & Session Storage**: The command outputs a new triage report containing the results. If a session workspace is configured, it persists the report in the session directory under `<workspace>/sessions/<session_id>/reports/<sequence>-<workflow>.json` to record the progress of the investigation.
