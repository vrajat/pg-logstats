# `pg-logstats top query-families`

`pg-logstats top query-families` ranks PostgreSQL query families inside one
bounded historical log window.

## Supported Mode

This workflow supports `log_backed` mode only.

At startup, `pg-logstats` requires a persisted `inspect` report. This workflow
then requires that the stored inspect report says `operating_mode =
"log_backed"`.

If the inspect report is missing, startup fails and points you to
`pg-logstats inspect`.

The inspect artifact is read from the workspace directory:

- default workspace: `~/.local/share/pg-logstats`
- override with `--workspace <dir>` or `PG_LOGSTATS_WORKSPACE`
- inspect artifact path: `<workspace>/inspect.json`

## Bounded Historical Window

The command works over the specific log window you provide:

- one or more local PostgreSQL log files
- `--log-dir` discovery
- AWS RDS / CloudWatch windows when using the RDS input path

The report includes:

- `analysis_window.since`
- `analysis_window.until`
- `source_summary.kind`
- `source_summary.entries_scanned`

That window is descriptive, not inferred from wall-clock state.

## Required Log Evidence

For useful ranking, the parser needs supported PostgreSQL statement and duration
evidence.

Common settings to verify:

- `log_destination = 'stderr'` for local stderr logs
- a `log_line_prefix` that preserves process identity
- duration logging enabled through PostgreSQL duration settings

Use `pg-logstats inspect` to determine whether the required evidence is present
in your environment and to persist the startup artifact required by later
commands.

## Supported Formats

Today this workflow is implemented for parser-supported PostgreSQL text logs:

- local PostgreSQL stderr logs
- AWS RDS PostgreSQL logs in the supported text shape

## Attribution

Each finding surfaces these attribution dimensions when available:

- `query_family.database`
- `query_family.user`
- `query_family.application_name`

When attribution is unavailable in the source logs:

- the specific field remains `null`
- `query_family.missing_attribution[]` lists the missing dimensions explicitly

Example:

```json
{
  "database": "appdb",
  "user": "app",
  "application_name": null,
  "missing_attribution": ["application_name"]
}
```

## References

Engineers used these references while shaping this workflow:

- pgBadger top-query style prior art: https://github.com/darold/pgbadger
- PostgreSQL logging configuration documentation: https://www.postgresql.org/docs/current/runtime-config-logging.html
