# Runbook Action Types

This page documents the detailed schema and statuses for the `next_actions[]` field in `pg-logstats` triage reports.

## Action Types

- `run_workflow`: Run another `pg-logstats` runbook directly.
- `run_sql`: Run a safe built-in SQL action through `pg-logstats run-sql`.
- `prompt_user`: Ask the operator to choose how the investigation should proceed.
- `stop`: End the current investigation branch.

## Priorities

- `required`
- `recommended`
- `optional`

## Next Action Status

- `allowed`: The action is safe to execute in the current state.
- `blocked_by_mode`: The action requires a different operating mode (e.g. `log_backed`).
- `blocked_by_verdict`: The database safety verdict blocks this action.
- `blocked_by_config`: The action has been disabled or exceeds `max_risk` in the configuration.
- `blocked_by_policy`: The action is blocked by built-in security policies.
- `omitted_not_enough_context`: The action requires missing identifiers (e.g., missing query ID).
- `omitted_unsupported_target`: The action target is not supported.

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
