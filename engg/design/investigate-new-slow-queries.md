# Design: Investigate New Slow Queries

Status: draft  
Date: 2026-04-19

## Overview

This design focuses on one workflow:

> Find slow query families that are new, newly important, or newly regressed, and tell the operator what to inspect next.

This is a strong candidate for the first compelling `pg-logstats` workflow because it is:

- easy to explain
- aligned with existing pgBadger usage
- useful to both humans and LLM agents
- naturally composable with `psql`, `pg_stat_statements`, and `auto_explain`

At the same time, it is harder to test than some neighboring workflows, so the design includes a staged implementation plan that reduces that risk.

## Problem Statement

Teams often have a stable set of “usual suspect” query families and want to notice when something changes:

- a query family becomes slow for the first time
- a normally minor query becomes operationally important
- a familiar query family regresses sharply relative to its usual performance

In the pgBadger ecosystem, this is already a real manual workflow. In issue `#872`, a user describes running pgBadger weekly, comparing top time-consuming and most-frequent normalized queries, and wanting a clear signal for which queries are new versus historically common.

Reference:

- <https://github.com/darold/pgbadger/issues/872>

The opportunity for `pg-logstats` is to turn that manual report-comparison loop into a tight CLI workflow that emits compact findings and suggested follow-up SQL.

## Goals

- Identify query families that are new or regressed in a target window.
- Rank findings so only the most relevant items are shown by default.
- Preserve evidence pointers to the underlying logs.
- Suggest next-step SQL for deeper live inspection.
- Keep output compact enough for CLI use and LLM use.

## Non-Goals

- Full SQL tuning automation
- Automatic index recommendation in the first version
- Full plan analysis in the first version
- Perfect statistical novelty detection in the first version
- Replacing `pg_stat_statements`, `EXPLAIN`, or `auto_explain`

## User Workflow

### Target CLI Shape

```text
pg-logstats update --logs /var/log/postgresql/*.jsonlog --cache .pg-logstats/
pg-logstats slow-queries diff --baseline last-7d --target today --top 20 --format llm-json
pg-logstats evidence qf_01J...
pg-logstats suggest-sql qf_01J...
```

### Operator Experience

1. ingest new logs into a local cache
2. compute query-family statistics for a target window
3. compare target window against a baseline
4. surface the top new or regressed query families
5. inspect evidence for one finding
6. run suggested SQL in `psql`

## Inputs

### Required

- PostgreSQL logs with statement and duration information

### Preferred

- `jsonlog`
- correlated statement and duration data

### Optional Companion Inputs

- `pg_stat_statements` snapshots
- `auto_explain` logs

These are optional for the first version, but the design should not block them.

## Output

The default output should be ranked findings, not raw log lines.

Example shape:

```json
{
  "schema_version": 1,
  "finding_id": "slow_qf_01J...",
  "kind": "new_slow_query_family",
  "rank": 1,
  "query_family_id": "qf_01J...",
  "title": "SELECT family newly slow in target window",
  "baseline": {
    "count": 4,
    "p95_ms": 90,
    "total_ms": 180
  },
  "target": {
    "count": 87,
    "p95_ms": 1120,
    "total_ms": 19430
  },
  "delta": {
    "p95_ms": 1030,
    "total_ms": 19250
  },
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

## Core Concepts

### Query Family

A query family is a normalized query identity used for grouping related executions.

Inputs for identity may include:

- normalized SQL
- `queryid`
- database
- user
- application name

The first version should avoid overfitting identity logic. It only needs to be stable enough for useful grouping.

### Baseline Window

The baseline is the historical comparison window.

Possible baseline modes:

- explicit previous window
- rolling N-day cache
- previous report snapshot

The first version should prefer explicit and deterministic comparisons over sophisticated adaptive baselines.

### Target Window

The target is the window being investigated now.

Example:

- “today”
- “last hour”
- one rotated log file

## Ranking Heuristic

The first version should use a simple heuristic, not a complex anomaly detector.

Possible signals:

- target `p95_ms`
- target `total_ms`
- target execution count
- delta in `p95_ms`
- delta in total runtime contribution
- query family absent or rare in baseline

Suggested first-pass eligibility rules:

- minimum target count
- minimum target total runtime
- minimum target or delta latency threshold

Then sort by a weighted regression score.

This will be less “clever” than a monitoring product, but much easier to explain and test.

## Evidence Model

Each finding should reference:

- representative source files
- a small number of sample events
- offsets or line references where possible
- the normalized query text or an abbreviated representation

This is important because the tool should not force an LLM or human to keep raw logs in memory just to trust a finding.

## Suggested SQL

Each finding should include a small set of follow-up SQL queries.

Examples:

- `pg_stat_statements` lookup for the same `queryid` or normalized text
- `pg_stat_activity` queries filtered by app or database
- if available, lookups for temp-file, lock, or I/O context

This is where `pg-logstats` hands off to `psql`.

## Why This Workflow Is Hard to Test

This workflow is harder than it first appears because it combines:

- parsing
- correlation
- grouping
- historical comparison
- ranking

The test risk is not only correctness. It is also whether the results feel believable and stable.

Specific difficulties:

- “newness” depends on baseline design
- “slowness” depends on thresholds
- ranking can become noisy with small sample sizes
- correlated statement/duration pairing must be reliable

## Staged Implementation Plan

### Stage 1: Top Slow Query Families in a Single Window

Do not start with novelty detection.

Start with:

- parse one window
- correlate statement and duration
- group into query families
- rank by target metrics only

This is much easier to test deterministically and exercises the same core data model.

### Stage 2: Two-Window Diff

Add:

- explicit baseline window
- explicit target window
- simple deterministic diff output

This should avoid hidden rolling baselines at first.

### Stage 3: New or Regressed Slow Queries

Add:

- novelty flags
- regression scoring
- compact explanation of why a query was surfaced

### Stage 4: Suggested SQL and Companion Correlation

Add:

- `suggest-sql`
- optional `pg_stat_statements` enrichment
- optional `auto_explain` enrichment

## Testing Strategy

### Deterministic Fixture Tests

Use small synthetic fixtures covering:

- one stable fast query family
- one stable slow query family
- one newly slow query family
- one noisy but low-count outlier

Expected output should assert:

- grouping correctness
- ranking order
- stable IDs
- evidence links

### Golden Output Tests

For compact JSON output:

- compare normalized JSON results
- ensure the schema is stable
- keep examples small enough to review by hand

### Integration Tests

Use:

- two log windows
- explicit baseline and target commands
- verification of top-ranked findings

### Companion Data Tests

Later stages can add:

- `pg_stat_statements` fixture imports
- `auto_explain` fixture correlation

## Simpler Neighboring Workflows

If this workflow proves too expensive to implement first, the best nearby stepping stones are:

1. top slow query families in one window
2. new error classes
3. top temp-file producers

These are documented in the investigations directory and should be evaluated as lower-risk first slices.

## Open Questions

- Should the first version require explicit baseline and target paths instead of named windows?
- Should “new” mean absent in baseline, or simply rare in baseline?
- How should low-count outliers be suppressed without hiding genuinely important findings?
- Should the first version expose raw weighted scores, or just human-readable reasons?
- How much should the ranking logic depend on total runtime versus latency percentile?
