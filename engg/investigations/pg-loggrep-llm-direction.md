# pg-logstats and LLM-First Investigation

Status: running investigation  
Date: 2026-04-19

## Summary

One promising direction for `pg-logstats` is to evolve into a CLI-first PostgreSQL log investigation tool that behaves more like `ripgrep` for operational workflows than like a traditional HTML report generator.

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

- `pg-logstats` narrows and structures the search space
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

`pg-logstats` should be positioned as:

- a PostgreSQL-native log investigation CLI
- optimized for fast drilldowns and structured output
- friendly to both humans and LLM-driven workflows
- complementary to `psql` instead of competing with it

The comparison should be:

- `ripgrep` for codebases
- `jq` for JSON
- `psql` for live PostgreSQL state
- `pg-logstats` for PostgreSQL log evidence and triage

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
pg-logstats top query-families --since 1h --format llm-json
pg-logstats errors --group-by sqlstate,app,user
pg-logstats temp-files --top 20
pg-logstats locks --since 30m
pg-logstats correlate --pg-stat-statements pgss.csv --auto-explain auto_explain.log
pg-logstats evidence <finding-id>
pg-logstats suggest-sql <finding-id>
pg-logstats diff --baseline yesterday.cache --target today.cache
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

## Concrete Workflows to Emulate

The most useful product clues are not abstract feature lists. They are the recurring manual workflows people already run with pgBadger and then inspect by hand.

### 1. Weekly "What Is New in Top Queries?" Review

This is the clearest direct workflow signal.

In pgBadger issue `#872`, the user describes running pgBadger weekly, comparing top time-consuming and most-frequent normalized queries, and wanting to know which queries are newly appearing versus always present.

Reference:

- <https://github.com/darold/pgbadger/issues/872>

This maps almost perfectly to an agent workflow:

1. refresh the local analysis cache
2. diff query families against previous windows
3. rank newly appearing or newly regressed families
4. attach evidence
5. suggest follow-up SQL

This is probably the best first product wedge for `pg-logstats`.

### 2. Hourly or Daily Incremental Analysis of Rotated Logs

Another concrete pattern is scheduled ingestion of new logs followed by inspection of the changed window.

In pgBadger issue `#697`, the user ingests RDS logs hourly, stores one-hour files, runs pgBadger in incremental mode, archives processed logs, and expects daily and weekly outputs to keep updating.

Reference:

- <https://github.com/darold/pgbadger/issues/697>

The underlying workflow is:

1. collect new logs
2. update rolling analysis state
3. inspect only what changed

This suggests an LLM-friendly model like:

- `pg-logstats update`
- `pg-logstats top`
- `pg-logstats diff`

without rebuilding a full report every time.

### 3. Estate-Wide Daily Triage Across Many Servers

Some users are not doing ad hoc single-file analysis. They are running pgBadger every day across fleets.

In pgBadger issue `#794`, the user runs pgBadger daily across multiple servers, excludes `pg_dump`, keeps a retention window, and uses incremental mode. The issue itself is about pgBadger getting OOM-killed, but the operational pattern matters more than the bug.

Reference:

- <https://github.com/darold/pgbadger/issues/794>

The real workflow is:

1. collect and process logs from many systems
2. suppress obvious maintenance noise
3. inspect the top findings from the fresh window

For `pg-logstats`, this argues strongly for:

- incremental caches
- cheap top-N retrieval
- low-memory summaries
- filters that remove known benign workloads before ranking findings

### 4. Scheduled Error-Only Review

The pgBadger documentation explicitly shows a cron workflow for weekly error reporting using watch mode.

Reference:

- <https://pgbadger.darold.net/documentation.html>

This manual pattern is simple and important:

1. scan the new log slice
2. group errors
3. collapse repeats
4. inspect the top classes by frequency or novelty

An agent-friendly version would make this much stronger by grouping on fields like:

- SQLSTATE
- application name
- user
- database
- normalized error text

and by emitting suggested follow-up SQL for live inspection.

### 5. Incremental Daily and Weekly Rolling Reports

The pgBadger docs also show incremental daily and weekly report generation with `-I`.

Reference:

- <https://pgbadger.darold.net/documentation.html>

That is effectively a rolling baseline workflow:

1. maintain state over time
2. update from newly rotated logs
3. inspect the latest day or week against historical context

For `pg-logstats`, the equivalent should probably be a local structured cache rather than HTML.

The key product question becomes:

- how do we materialize investigative state cheaply enough that both humans and agents can query it interactively?

### 6. Noise Suppression Before Ranking

The pgBadger docs explicitly show excluding `pg_dump` windows or app names so that `COPY`-heavy maintenance activity does not dominate the slowest-query view.

Reference:

- <https://pgbadger.darold.net/documentation.html>

This is a very real operational workflow:

1. remove known background jobs
2. re-rank the remaining workload
3. inspect true anomalies

This matters because real investigations are not "search everything." They are "search after subtracting known noise."

That should become a first-class concept in `pg-logstats`, not an afterthought.

### 7. Cross-Tier PostgreSQL + PgBouncer Investigation

The pgBadger docs show parsing a local PostgreSQL log together with a remote PgBouncer log in one run.

Reference:

- <https://pgbadger.darold.net/documentation.html>

This suggests a valuable investigation loop:

1. identify slowness or queueing symptoms
2. inspect both PostgreSQL-side and pooler-side evidence
3. decide whether the bottleneck is pool saturation, server execution, or both

That kind of cross-layer correlation is exactly the sort of thing an LLM agent can help navigate if the CLI emits compact evidence.

### 8. Slow Query to Plan Review

pgBadger 13.0 added support for `auto_explain` plans in CSV and JSON logs.

Reference:

- <https://www.postgresql.org/about/news/pgbadger-130-released-2975/>

This implies a concrete workflow:

1. detect the problematic query family
2. inspect representative slow executions
3. pull the plan
4. decide whether the next step is indexing, SQL rewrite, or live database inspection

This is one of the strongest LLM-assisted workflows because the agent can:

- summarize the query family
- summarize the plan shape
- propose the next `psql` queries or `EXPLAIN` targets

## Product Implication

The manual pgBadger workflow is usually:

1. scheduled ingestion
2. manual inspection of top sections
3. ad hoc follow-up SQL

The agentified `pg-logstats` workflow should be:

1. incrementally ingest
2. rank deltas and new findings
3. suppress known noise
4. attach evidence
5. generate the next SQL to run

That is a much sharper and more concrete product target than “AI for Postgres” in the abstract.

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

- Should the ripgrep comparison remain just a positioning concept inside `pg-logstats`, or should it shape subcommand naming too?
- Should the first output schema be JSONL, compact JSON, or both?
- Should local caching be visible and user-managed, or implicit?
- How much of the first version should depend on `pg_stat_statements` being available?
- Is diffing baseline versus target a core workflow or a later feature?
