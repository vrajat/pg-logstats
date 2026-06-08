# Readiness

`pg-logstats readiness` is the first command to run before deeper
investigation. It tells the caller which operating mode is actually supported
by the available PostgreSQL evidence.

## Purpose

The command answers four questions:

1. Can `pg-logstats` prove `log_backed` mode from real log evidence?
2. If not, is `live_only` mode available from PostgreSQL system views?
3. If neither is true, why is the environment `unready`?
4. What command should the caller run next?

## Connection Resolution

Live PostgreSQL checks use this precedence:

1. `--dsn <postgres-url>`
2. `PG_LOGSTATS_DATABASE_URL`
3. `[database].dsn` from the resolved config file

`--dsn` is a global CLI flag, not a readiness-only option.

## Supported Evidence

`log_backed` currently requires parsed statement and duration evidence from:

- local PostgreSQL stderr-style logs
- AWS RDS text logs supported by the current text parser

Documented but not yet `log_backed`-ready:

- `csvlog`
- `jsonlog`

`live_only` currently requires successful PostgreSQL probes for:

- `track_activities`
- `shared_preload_libraries` including `pg_stat_statements`
- `compute_query_id`
- `pg_stat_statements` extension presence
- `pg_read_all_stats` visibility or equivalent superuser access
- lightweight access to `pg_stat_activity`
- lightweight access to `pg_stat_statements`

## Output Shape

The JSON output is a `PgTriageReport` with `workflow = "readiness"` and
`payload.readiness` containing the readiness-specific fields.

Example:

```json
{
  "schema_version": 1,
  "workflow": "readiness",
  "operating_mode": "log_backed",
  "limitations": ["live_database_checks_unavailable"],
  "payload": {
    "readiness": {
      "database_readiness": {
        "mode_candidate": "log_backed",
        "checks": {
          "log_source_reachable": {
            "status": "passed",
            "value": 9
          },
          "statement_evidence": {
            "status": "passed",
            "value": true
          },
          "duration_evidence": {
            "status": "passed",
            "value": true
          },
          "pg_stat_activity_probe": {
            "status": "skipped",
            "reason": "database_connection_not_configured"
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
      "required_checks": [
        "log_source_reachable",
        "statement_evidence",
        "duration_evidence",
        "correlation_evidence",
        "log_destination",
        "log_line_prefix",
        "log_duration",
        "log_min_duration_statement",
        "log_temp_files",
        "track_activities",
        "shared_preload_libraries",
        "compute_query_id",
        "pg_stat_statements_extension",
        "pg_read_all_stats",
        "pg_stat_activity_probe",
        "pg_stat_statements_probe"
      ],
      "failed_checks": [],
      "recommended_next_commands": [
        "pg-logstats top query-families --output-format json"
      ]
    }
  }
}
```

## Check Semantics

Each readiness check has:

- `status`: `passed`, `failed`, or `skipped`
- `value`: optional observed value
- `reason`: optional machine-readable reason code

Important behavior:

- missing DB connection marks live checks as `skipped`
- requested but unusable log input marks `log_source_reachable` as `failed`
- `failed_checks` lists the machine-readable reason codes for failed checks
- `limitations` describes what the caller must not assume in the chosen mode

## Modes

### `log_backed`

Chosen when parsed log evidence proves:

- log source reachable
- statement evidence present
- duration evidence present
- statement/duration correlation present

Typical next step:

```bash
pg-logstats top query-families --output-format json ...
```

### `live_only`

Chosen when log-backed evidence is unavailable but all required PostgreSQL live
checks pass.

Typical next step:

```bash
pg-logstats running-queries --output-format json
```

### `unready`

Chosen when neither `log_backed` nor `live_only` can be supported honestly.

Typical fixes:

- provide a supported log source
- provide a resolvable PostgreSQL connection
- enable the required PostgreSQL monitoring settings

## Text Output

The text form is intentionally simple and stable. It is not a full TUI. The
longer-term direction should be a shared CLI text-report layer once multiple
commands need the same table or section rendering model.
