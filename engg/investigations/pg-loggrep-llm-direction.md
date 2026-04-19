# pg-loggrep and LLM-First Investigation

Status: running investigation  
Date: 2026-04-19

## Summary

One promising direction for `pg-logstats` is to evolve it into **`pg-loggrep`**: a CLI-first PostgreSQL log investigation tool that behaves more like `ripgrep` for operational events than like a traditional HTML report generator.

The key difference is that this should not be plain regex over log lines. It should parse PostgreSQL logs into structured events and return compact, investigation-friendly output that can be combined with:

- `psql`
- PostgreSQL system views
- `pg_stat_statements`
- `auto_explain`
- shell tools like `jq`, `fzf`, `xargs`, and `make`

This direction is especially attractive for LLM-assisted workflows because the bottlenecks are:

1. speed
2. token budget
3. preserving evidence without dumping raw logs

## The Core Hypothesis

`psql` is already a powerful interface for querying current database state.

`pgBadger` is already a powerful interface for broad PostgreSQL log reporting.

The open space is a tool that turns PostgreSQL logs into **small, structured diagnostic packets** that help a human or agent answer:

- what changed?
- what is suspicious?
- what should I inspect next?
- which SQL should I run now?

That is a more useful target than trying to replace `psql` or out-feature `pgBadger`.

## Why `psql` Is Not Enough

`psql` is excellent for:

- live database queries
- system catalog inspection
- joining PostgreSQL views and extension tables
- exporting or scripting SQL output

Relevant docs:

- <https://www.postgresql.org/docs/current/app-psql.html>
- `\copy`, `COPY ... TO STDOUT`, `\g`, `-c`, and `-f` make it very scriptable

But `psql` is not designed to do these jobs well:

- parse rotated log files
- handle `stderr`, `csvlog`, and `jsonlog` uniformly
- normalize query families from raw log streams
- correlate statement, duration, error, and plan events from logs
- rank historical events before deciding which SQL to run

So the right model is:

- `pg-loggrep` narrows and structures the search space
- `psql` performs the deeper live interrogation

## Why `pgBadger` Is Not Enough

`pgBadger` already covers an impressive surface area:

- `stderr`, `csvlog`, and `jsonlog`
- compressed files
- remote logs and URI-based inputs
- parallel processing
- incremental daily and weekly reports
- JSON output
- query reports, sessions, connections, locks, temp files, checkpoints, autovacuum
- PgBouncer support
- `auto_explain` support, including recent support for plans in CSV and JSON logs

References:

- <https://pgbadger.darold.net/documentation.html>
- <https://www.postgresql.org/about/news/pgbadger-130-released-2975/>

This matters because it rules out the lazy strategy of just rebuilding pgBadger in Rust.

The better wedge is not breadth. It is **machine-first investigation**.

## Proposed Product Position

`pg-loggrep` should be positioned as:

- a PostgreSQL-native log investigation CLI
- optimized for fast drilldowns and structured output
- friendly to both humans and LLM-driven workflows
- complementary to `psql` instead of competing with it

The comparison should be:

- `ripgrep` for codebases
- `jq` for JSON
- `psql` for live PostgreSQL state
- `pg-loggrep` for PostgreSQL log evidence and triage

## What the Tool Should Add Above `psql + SQL`

### 1. Historical Context

The tool should operate on log files and log directories, not just on current server state.

This includes:

- rotated logs
- sampled logs
- incident windows
- exported customer logs
- partial or offline evidence

### 2. Query Family Identity

The tool should normalize noisy queries into stable-enough local identities for investigation.

It should not rely only on raw query text.

It should use, when available:

- normalized SQL
- `queryid`
- app name
- user
- database
- process or session metadata

Note:

`pg_stat_statements` is useful here, but PostgreSQL documents that `queryid` has only limited stability guarantees across environments and major versions.

Reference:

- <https://www.postgresql.org/docs/current/pgstatstatements.html>

### 3. Cross-Signal Correlation

The tool should connect:

- statement logs
- duration logs
- error logs
- `auto_explain` plans
- companion snapshots such as `pg_stat_statements`

That is more valuable than counting lines independently.

### 4. Ranked Findings

Instead of returning “all matching lines,” the tool should support outputs like:

- top slow query families
- top new error classes
- top temp-file producers
- top lock-related events
- top workload shifts versus a baseline

This is what reduces token usage for LLMs.

### 5. Evidence-Preserving Output

Every finding should be traceable back to evidence without requiring the user or agent to keep full raw logs in context.

Useful fields include:

- source file
- line number or byte offset
- time window
- sample count
- representative raw event references
- confidence notes
- sampling metadata

### 6. Follow-Up SQL

The CLI should often end with suggested next-step SQL:

- `pg_stat_statements` lookups
- lock table inspection
- `pg_stat_activity` filters
- temp-file or autovacuum related queries

This is one of the strongest complements to `psql`.

## LLM Constraints

### Speed

The tool should be fast enough to sit in a tight agent loop.

Implications:

- streaming parsers
- incremental local cache
- top-N outputs by default
- avoid full-report generation for common paths

### Token Budget

The tool should default to small, dense outputs.

Implications:

- one line or one JSON object per finding
- stable identifiers instead of repeating giant query strings
- explicit compact modes such as `--format llm-json`
- shared metadata blocks rather than repeated fields everywhere

### Determinism

Agents benefit from output that is consistent across runs.

Implications:

- stable sort order
- explicit schemas
- versioned output format
- low-noise summaries

## Recommended CLI Shape

The CLI should feel closer to search and triage than to report generation.

Candidate commands:

```text
pg-loggrep top query-families --since 1h --format llm-json
pg-loggrep errors --group-by sqlstate,app,user
pg-loggrep temp-files --top 20
pg-loggrep locks --since 30m
pg-loggrep correlate --pg-stat-statements pgss.csv --auto-explain auto_explain.log
pg-loggrep evidence <finding-id>
pg-loggrep suggest-sql <finding-id>
pg-loggrep diff --baseline yesterday.cache --target today.cache
```

## Output Shape

The best output shape is likely neither raw text nor giant nested JSON.

A better default is a compact, flat, machine-readable schema like:

```json
{
  "schema_version": 1,
  "finding_id": "qf_01J...",
  "kind": "query_family",
  "rank": 1,
  "title": "SELECT family latency regression",
  "query_family_id": "qf_01J...",
  "queryid": "1234567890",
  "database": "appdb",
  "user": "app",
  "application_name": "api",
  "count": 184,
  "p95_ms": 842,
  "delta_vs_baseline_ms": 510,
  "evidence": {
    "files": ["postgresql-2026-04-19.jsonlog"],
    "sample_events": 3
  },
  "next_sql": [
    "select * from pg_stat_statements where queryid = 1234567890;",
    "select pid, wait_event_type, wait_event, query from pg_stat_activity where application_name = 'api';"
  ]
}
```

## Important Technical Inputs

PostgreSQL logging itself already supports some of the right primitives:

- `stderr`
- `csvlog`
- `jsonlog`
- sampled duration logging with `log_min_duration_sample`
- stochastic logging via `log_statement_sample_rate`

References:

- <https://www.postgresql.org/docs/current/runtime-config-logging.html>

PostgreSQL extensions also help:

- `pg_stat_statements`
- `auto_explain` with `json` format

Reference:

- <https://www.postgresql.org/docs/current/auto-explain.html>

## Risks

### Risk 1: Becoming a Generic Log Search Tool

That would dilute the strongest differentiator: PostgreSQL-specific semantics.

### Risk 2: Becoming Another Report Generator

If the tool drifts toward static HTML summaries, it competes directly with pgBadger on weaker ground.

### Risk 3: Overfitting to LLMs

The tool still needs to be useful as a human CLI. LLM-friendliness should improve the CLI, not make it unnatural.

## Recommended Near-Term Direction

If this direction is pursued, the first concrete implementation target should be:

1. define a normalized event model
2. support `jsonlog` first
3. build a compact top-findings CLI
4. add evidence references
5. add `suggest-sql` as a thin bridge into `psql`

That sequence keeps the product grounded in real workflows rather than speculative AI tooling.

## Open Questions

- Should `pg-loggrep` be a rename, a subcommand family, or just a positioning concept inside `pg-logstats`?
- Should the first output schema be JSONL, compact JSON, or both?
- Should local caching be visible and user-managed, or implicit?
- How much of the first version should depend on `pg_stat_statements` being available?
- Is diffing baseline versus target a core workflow or a later feature?
