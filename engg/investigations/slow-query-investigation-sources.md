# Slow Query Investigation Sources

Status: investigation  
Date: 2026-04-19

## Why This Exists

This note collects concrete references for how people investigate slow PostgreSQL queries in practice. The point is not to build a giant bibliography. It is to capture obvious recurring patterns that `pg-logstats` can emulate.

## Sources and Observations

### 1. pgBadger issue #872: weekly comparison of top queries

Reference:

- <https://github.com/darold/pgbadger/issues/872>

Observed workflow:

- run pgBadger weekly
- compare top time-consuming and most-frequent normalized queries
- identify which query families are new or repeatedly present

This is the clearest direct precedent for a “new slow queries” workflow.

### 2. pgBadger docs: incremental daily and weekly reporting

Reference:

- <https://pgbadger.darold.net/documentation.html>

Observed workflow:

- ingest rotated logs incrementally
- maintain daily or weekly report state
- inspect rolling changes instead of re-reading everything manually

This is the right operational pattern for a cache-backed CLI.

### 3. pgBadger docs: watch mode for error review

Reference:

- <https://pgbadger.darold.net/documentation.html>

Observed workflow:

- scan new log slices repeatedly
- produce a focused report for a narrow investigation domain

This supports the idea that smaller, targeted investigation commands are more useful than giant monolithic reports.

### 4. PostgreSQL wiki: Logging Difficult Queries

Reference:

- <https://wiki.postgresql.org/wiki/Logging_Difficult_Queries>

Observed workflow:

- use `log_min_duration_statement` to capture slow queries
- optionally use `pg_stat_statements`
- use tools like pgBadger to sort through the data
- use `auto_explain` when plans matter

This confirms that slow-query investigation already combines logs, aggregate statistics, and plans.

### 5. PostgreSQL wiki: Slow Query Questions

Reference:

- <https://wiki.postgresql.org/wiki/SlowQueryQuestions>

Observed workflow:

- gather `EXPLAIN (ANALYZE, BUFFERS, SETTINGS)`
- check statistics freshness and bloat
- inspect GUCs and hardware context
- enable targeted logging such as:
  - `log_min_duration_statement`
  - `log_checkpoints`
  - `log_lock_waits`
  - `log_temp_files`
  - `csvlog`

This is valuable because it shows that “investigate slow queries” quickly expands into a checklist of related evidence.

### 6. PostgreSQL docs: auto_explain

Reference:

- <https://www.postgresql.org/docs/current/auto-explain.html>

Observed workflow:

- log the plans of slow statements automatically
- investigate the actual plan used at the time of slowness

This is critical because rerunning `EXPLAIN` later may not show the same plan or runtime context.

### 7. Aiven docs: identify slow queries with pg_stat_statements

Reference:

- <https://aiven.io/docs/products/postgresql/howto/identify-pg-slow-queries>

Observed workflow:

- use `pg_stat_statements` to rank slow and expensive queries by aggregated statistics

This supports using `pg_stat_statements` as a natural follow-up surface after log-driven triage.

### 8. CYBERTEC: detecting slow queries quickly

Reference:

- <https://www.cybertec-postgresql.com/en/postgresql-detecting-slow-queries-quickly/>

Observed workflow:

- use `pg_stat_statements` to isolate bottlenecks first
- focus on query-level visibility before tuning

This reinforces that slow-query investigation usually starts with ranking, not with raw log browsing.

### 9. pganalyze docs and blog: auto_explain and historical query analysis

References:

- <https://pganalyze.com/docs/explain/setup>
- <https://pganalyze.com/blog/5mins-postgres-debug-UPDATE-bloated-tables-auto-explain-pageinspect>

Observed workflow:

- use auto-explain to capture the actual plan from the time the slow query happened
- avoid relying only on re-running EXPLAIN after the fact
- connect query statistics with captured plans and context

This is very close to the long-term `pg-logstats` vision of logs plus companion data plus next-step analysis.

## Summary

The recurring slow-query investigation pattern is:

1. collect slow-query evidence from logs or query statistics
2. rank or group the important query families
3. compare against historical expectations where possible
4. inspect plans and context for the top findings
5. run follow-up SQL in the live system

That is a strong fit for `pg-logstats`.
