# `pg-logstats errors`

`pg-logstats errors` triages, groups, and attributes PostgreSQL error log events inside a bounded historical log window.

## Supported Mode

This workflow supports `log_backed` mode only.

At startup, `pg-logstats` requires a persisted `inspect` report. This workflow then requires that the stored inspect report says `operating_mode` is either `log_backed` or `log_backed_and_live`.

If the inspect report is missing, startup fails and points you to `pg-logstats inspect`.

The inspect artifact is read from the workspace directory:
- default workspace: `~/.local/share/pg-logstats`
- override with `--workspace <dir>` or `PG_LOGSTATS_WORKSPACE`
- inspect artifact path: `<workspace>/inspect.json`

## Bounded Historical Window

The command parses errors over the specific log window you provide:
- one or more local PostgreSQL log files
- `--log-dir` discovery
- AWS RDS / CloudWatch windows when using the RDS input path

The report includes standard historical metadata:
- `analysis_window.since`
- `analysis_window.until`
- `source_summary.kind`
- `source_summary.scanned_entries`

## Required Log Evidence & Parsing

The parser identifies errors by classifying `ERROR`, `FATAL`, and `PANIC` level entries:
- **SQLSTATE Extraction**: Since standard PostgreSQL stderr logs may not output SQLSTATE explicitly unless prefix configured, the tool automatically parses SQLSTATE codes from log messages starting with a 5-character alphanumeric prefix (e.g. `42P01: relation "users" does not exist`).
- **Classification**: Normalizes and classifies messages into `ErrorClassFinding` payloads.

## Normalization & Deduplication

To group identical errors together despite having different IDs, values, or strings, the tool applies message normalization:
- Double-quoted strings (`"..."`) become `?"?`
- Single-quoted strings (`'...'`) become `'?'`
- Raw digits/numbers become `?`
- IPv4 addresses (e.g. `127.0.0.1`) become `?.?.?.?`

For example:
`connection to 10.0.0.5 failed on port 5432` 
is normalized to:
`connection to ?.?.?.? failed on port ?`

## Attribution & Output Format

Each finding outputs the following fields when available:
- `sqlstate`: The extracted SQLSTATE code (e.g. `42P01`, `3D000`).
- `normalized_error`: The normalized error message used for deduplication.
- `database`: Database where the error occurred.
- `user`: User executing the query.
- `application_name`: Application name from connection properties.
- `error_count`: Total occurrences of this error class in the log window.

Example finding payload:
```json
{
  "id": "42P01",
  "kind": "error_class",
  "rank": 1,
  "title": "relation \"?\" does not exist",
  "reason": "Occurred 12 times in the log window",
  "score": 12.0,
  "confidence": "high",
  "error_class": {
    "sqlstate": "42P01",
    "normalized_error": "relation \"?\" does not exist",
    "database": "appdb",
    "user": "app",
    "application_name": "api",
    "error_count": 12
  }
}
```

## Diagnostic Next Actions

If the triage report connects to a database in `log_backed_and_live` mode, it emits next actions to run follow-up diagnostic SQL query:
- `error_class.pg_stat_activity.by_dimensions`: Returns active backends matching the specific `database`, `user`, and `application_name` of the error class to pinpoint active queries causing issues.
