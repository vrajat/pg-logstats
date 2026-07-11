---
title: pg-logstats Action Types and Safety Policies
description: Review pg-logstats action types, safety verdicts, and bounded SQL policies that control what coding agents can do during PostgreSQL triage.
schema:
  "@context": "https://schema.org"
  "@type": "TechArticle"
  headline: "pg-logstats Action Types and Safety Policies"
  description: "Safety policies, action classes, and bounded SQL controls for pg-logstats investigations."
  url: "https://pg-logstats.vrajat.com/runbooks/action-types/"
---

# Investigation Guidance & Safety Policies

`pg-logstats` models the database triage process as a directed acyclic investigation graph (DAG) with developer or agent judgement at branch points. 

Instead of requiring callers to invent database-specific diagnostic commands, every triage report includes a list of safe, contextual `next_actions[]` that the caller can select from.

Some next actions are directly executable. Others are delegated branch points where the agent must ask the operator for a decision, such as whether to provide a DSN and rerun `inspect`.

---

## Next Action Structure

Every machine-readable triage report (JSON output) includes a top-level `next_actions[]` field. Each next action in the list has the following shape:

```json
{
  "action_id": "query_family.pg_stat_activity.by_dimensions:qf_51125b8829ab1fdf",
  "action_type": "run_sql",
  "label": "Find current active sessions for the same query-family dimensions",
  "status": "allowed",
  "priority": "optional",
  "judgement_required": true,
  "reason": "The finding includes database, user, or application attribution that can bound pg_stat_activity.",
  "target_id": "qf_51125b8829ab1fdf",
  "command": {
    "argv": ["pg-logstats", "--triage-report", "20260613T181530123456Z-top_query_families", "--action-id", "query_family.pg_stat_activity.by_dimensions:qf_51125b8829ab1fdf", "run-sql"]
  },
  "risk": "safe",
  "action_class": "bounded_activity_queries"
}
```

---

## Action Schema Details

### Action Types

- `run_workflow`: Run another `pg-logstats` runbook directly.
- `run_sql`: Run a safe built-in SQL action through `pg-logstats run-sql`.
- `prompt_user`: Ask the operator to choose how the investigation should proceed.
- `stop`: End the current investigation branch.

### Priorities

- `required`
- `recommended`
- `optional`

### Next Action Status

- `allowed`: The action is safe to execute in the current state.
- `blocked_by_mode`: The action requires a different operating mode (e.g. `log_backed_only`, `log_backed_and_live`, or `live_only`).
- `blocked_by_verdict`: The database safety verdict blocks this action.
- `blocked_by_config`: The action has been disabled or exceeds `max_risk` in the configuration.
- `blocked_by_policy`: The action is blocked by built-in security policies.
- `omitted_not_enough_context`: The action requires missing identifiers (e.g., missing query ID).
- `omitted_unsupported_target`: The action target is not supported.

---

## Delegated Operator Actions

When a report cannot proceed safely on its own, `pg-logstats` may emit a delegated `prompt_user` action instead of a runnable SQL action.

Example:

```json
{
  "action_id": "workspace.prompt_user.enable_live_follow_up",
  "action_type": "prompt_user",
  "label": "Enable live follow-up or stop",
  "status": "allowed",
  "priority": "recommended",
  "reason": "This investigation ranked historical findings from logs only. Live follow-up requires a configured DSN and a fresh inspect run.",
  "survey": {
    "question": "How should the investigation proceed?",
    "choices": [
      {
        "choice_id": "configure_dsn_and_rerun_inspect",
        "label": "Configure DSN and rerun inspect",
        "description": "Provide database access for this workspace so pg-logstats can unlock live SQL follow-up.",
        "workflow": "inspect",
        "command": {
          "argv": ["pg-logstats", "inspect"]
        }
      },
      {
        "choice_id": "stop_with_offline_findings",
        "label": "Stop with offline findings",
        "description": "End the investigation after offline log triage without enabling live database access.",
        "workflow": "stop"
      }
    ]
  }
}
```

The important rule is:

- only `action_type = "run_sql"` should be executed through `pg-logstats run-sql`
- `action_type = "prompt_user"` means the agent must ask the operator for a decision first

---

## Interpreted SQL Results

`run-sql` reports can include a small `payload.insights[]` list when `pg-logstats` recognizes a strong pattern in the bounded result set of one of its own built-in SQL actions.

Example:

```json
{
  "workflow": "run_sql",
  "payload": {
    "action_id": "query_family.pg_stat_activity.by_dimensions:qf_51125b8829ab1fdf",
    "source_report_id": "20260613T181530123456Z-top_query_families",
    "source_finding_id": "qf_51125b8829ab1fdf",
    "insights": [
      {
        "insight_id": "transactionid_lock_wait",
        "label": "The query appears blocked on another transaction",
        "confidence": "high",
        "reason": "A matching active session is waiting on wait_event_type=Lock and wait_event=transactionid."
      }
    ],
    "row_count": 1,
    "truncated": false,
    "columns": ["pid", "usename", "datname", "application_name", "state", "wait_event_type", "wait_event", "query_start", "query_id", "query"],
    "rows": [[42137, "app", "appdb", "api", "active", "Lock", "transactionid", "2026-06-14T10:00:18+00:00", null, "SELECT * FROM invoices WHERE workspace_id = $1 ORDER BY created_at DESC LIMIT $2"]]
  }
}
```

Current product rule:

- `source_finding_id` ties the live follow-up back to the parent finding
- `insights[]` is emitted only when the built-in action result supports a bounded interpretation
- an empty `insights[]` is valid; `pg-logstats` should not invent certainty from weak or ambiguous SQL output

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

## Executing SQL Actions with Linkage Flags

For `action_type = "run_sql"`, the caller executes `pg-logstats run-sql` and links it to the parent report using global audit flags.

### Command Usage Example
```bash
pg-logstats \
  --triage-report 20260613T181530123456Z-top_query_families \
  --action-id query_family.pg_stat_activity.by_dimensions:qf_51125b8829ab1fdf \
  run-sql
```

### Global Audit Linkage Options
- `--triage-report <REPORT>`: The persisted report that led to this follow-up action. Accepts a report ID or a report path.
- `--action-id <ACTION_ID>`: The `action_id` from the parent report's `next_actions` array.

### Behavior & Security
1. **Safety Re-evaluation**: `pg-logstats` reads the parent report, finds the requested action, and re-validates the policy matrix against the current state and parameters. If the action is blocked, unknown, or not a SQL action, execution is rejected with a structured error.
2. **Execution**: The subcommand (e.g. `run-sql`) is executed with safety checks in place.
3. **Report Output & Persistence**: The command outputs a new triage report containing the results. Follow-up actions persist reports under `<workspace>/reports/<timestamp>-<runbook>.json` so the investigation history remains auditable without overwriting prior steps.

---

## Built-In SQL Actions

The gateway provides a set of pre-approved, built-in SQL actions for query-family and temp-file triage:

| Rule ID | Purpose | Required identifiers | Risk | Action class | Attribution |
|---|---|---|---|---|---|
| `query_family.pg_stat_statements.by_queryid` | Exact `pg_stat_statements` lookup for the query family. | `queryid` | `safe` | `stats_view_reads` | PostgreSQL `pg_stat_statements` exact queryid lookup |
| `query_family.pg_stat_activity.by_dimensions` | Bounded `pg_stat_activity` lookup using the finding's database, user, and application attribution. | at least one of `database`, `user`, `application_name` | `safe` when `application_name` is present, otherwise `bounded` | `bounded_activity_queries` | PostgreSQL `pg_stat_activity` lookup by app, database, and user |
| `query_family.explain` | Explain query execution plan for query family. | (None) | `safe` | `explain_without_analyze` | PostgreSQL EXPLAIN query plan |
| `query_family.explain_analyze` | Explain analyze query execution plan for query family. | (None) | `bounded` | `explain_analyze` | PostgreSQL EXPLAIN ANALYZE BUFFERS query plan |
| `temp_file.pg_stat_database.temp_counters` | Check database temp counters in pg_stat_database. | `database` | `safe` | `stats_view_reads` | PostgreSQL pg_stat_database counters lookup |
| `temp_file.pg_stat_statements.temp_blocks` | Check pg_stat_statements temp block activity. | (None) | `safe` | `stats_view_reads` | PostgreSQL pg_stat_statements temp blocks lookup |
| `temp_file.explain` | Explain query execution plan for the temp-file query. | (None) | `safe` | `explain_without_analyze` | PostgreSQL EXPLAIN query plan for temp files |
| `temp_file.explain_analyze` | Explain analyze query execution plan for the temp-file query. | (None) | `bounded` | `explain_analyze` | PostgreSQL EXPLAIN ANALYZE BUFFERS query plan for temp files |

`run-sql` executes only built-in SQL actions selected from a parent report. The caller can supply `--parameter NAME=VALUE`, but cannot supply raw SQL text.

---

## Attribution

The runbook-level attribution lives in [engg/design/internal-ai-app-triage-v1.md](https://github.com/vrajat/pg-logstats/blob/main/engg/design/internal-ai-app-triage-v1.md), especially the "Workflow Attribution And Selection" section and its initial V1 reference map.

For the query-family SQL actions in this phase, the intended prior art is:

- PostgreSQL `pg_stat_statements` documentation for exact `queryid` lookup and normalized statement identity.
- PostgreSQL `pg_stat_activity` documentation for active-session inspection by database, user, application name, wait state, and `query_id`.
- pgBadger top-query reports as report-shape prior art for carrying query text plus attribution dimensions into follow-up investigation.
