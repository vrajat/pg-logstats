# `pg-logstats run-action` and Investigation Guidance

`pg-logstats` models the database triage process as a directed acyclic investigation graph (DAG) with developer or agent judgement at branch points. 

Instead of requiring callers to invent database-specific diagnostic commands, every triage report includes a list of safe, contextual `next_actions[]` that the caller can select from. The caller then executes the chosen action using the `run-action` command.

---

## The Investigation Guidance Framework

Every machine-readable triage report (JSON output) includes a top-level `next_actions[]` field. Each next action in the list has the following shape:

```json
{
  "action_id": "query_family.pg_stat_statements.lookup:query_family:queryid=|db=appdb|user=app|app=api|sql=SELECT * FROM users WHERE id = ?",
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
- `run_pg_logstats`: Runs another `pg-logstats` command/workflow.
- `run_sql`: Runs a safe, built-in diagnostic SQL query.
- `install_agent_guidance`: Installs agent skill playbooks/harnesses.
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

## Executing Actions: `run-action`

The `run-action` command is an executor for actions suggested by reports. The caller chooses an allowed action from `next_actions[]` and executes it.

### Command Usage
```bash
pg-logstats run-action --report <report-json-path> --action-id <action-id> [options]
```

### Options
- `--report <PATH>`: The source report JSON file path.
- `--action-id <ID>`: The `action_id` from the source report's `next_actions` array.
- `--dsn <postgres-url>`: (Optional) Database URL to use when executing `run_sql` actions.
- `--quiet`: Suppress diagnostic messages.

### Behavior & Security
1. **Safety Re-evaluation**: `pg-logstats` reads the report, finds the requested action, and re-validates the policy matrix. If the action is blocked or unknown, execution is rejected with a structured error.
2. **Parameter Binding**: For `run_sql` actions, the command automatically binds parameters (such as `queryid` or `normalized_sql`) from the findings in the source report into the SQL query template.
3. **Execution**: The built-in SQL query is executed against the database using the configured DSN.
4. **Report Output & Session Storage**: The command outputs a new triage report containing the query results. If a session workspace is configured, it persists the report in the session directory to record the progress of the investigation.

---

## Sessions & Investigation Replay

When session storage is enabled, `pg-logstats` writes all generated reports in the workspace directory under a specific session:

- Reports are saved at: `<workspace>/sessions/<session_id>/reports/<sequence>-<workflow>.json`
- Session results are saved at: `<workspace>/sessions/<session_id>/results/`

Each persisted report contains session metadata to allow auditability and replay:

- `report_id`: Unique identifier for the report.
- `session_id`: Unique identifier for the current investigation session.
- `parent_report_id`: The ID of the report that led to this action (null for the initial `inspect` report).
- `selected_action_id`: The ID of the action that was executed to produce this report.
- `created_at`: The RFC3339 timestamp when the report was generated.

This metadata enables deterministic replay and analysis of the investigation path.
