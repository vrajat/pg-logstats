# ADR 0002: Select three investigation workflows for V1

- Status: Accepted
- Date: 2026-04-22

## Context

`pg-logstats` is being repositioned as a PostgreSQL-native investigation CLI rather than a broad report generator.

The current codebase is still early for that direction:

- it is CLI-first and useful today
- it is centered on `stderr` parsing
- it emits summary-style output
- statement and duration correlation is still weak

That means V1 cannot responsibly chase a wide `pgBadger` surface area. It needs a small set of workflows that:

- match real PostgreSQL operator behavior
- are strong fits for compact, evidence-preserving output
- are realistic on top of the current implementation plan

The main workflow signals in the current project docs and external pgBadger discussion are consistent:

- top query analysis remains a primary troubleshooting loop
- grouped error and event review is a routine operational task
- temp-file inspection is one of the most practically useful PostgreSQL-specific views

At the same time, the repo's own investigations conclude that baseline diffing, lock analysis, broad reporting, and other cross-signal workflows are better follow-on work after the event model and correlation layer are stable.

## Decision

V1 will implement exactly these three investigation workflows:

1. slow query family triage in a single target window
2. grouped error and event triage
3. temp-file triage

These workflows map to the likely first command surface:

- `pg-logstats top query-families`
- `pg-logstats errors`
- `pg-logstats temp-files`

### 1. Slow Query Family Triage In A Single Target Window

This is the V1 version of the longer-term "new slow queries" direction.

Why it is in scope:

- it matches how operators already use pgBadger's top query views
- it exercises the most important core capability: statement plus duration correlation
- it avoids the added complexity of baselines, novelty scoring, and cache management in the first cut

V1 behavior:

- group executions into query families
- rank by target-window latency and runtime contribution
- preserve evidence pointers
- emit follow-up SQL for `pg_stat_statements` and `pg_stat_activity` when possible

### 2. Grouped Error And Event Triage

This is the most direct operational workflow after query triage.

Why it is in scope:

- it maps cleanly to pgBadger's "Events" style usage
- it is easier to parse and test deterministically than slow-query diffing
- it gives LLMs compact failure-oriented packets instead of raw log dumps

V1 behavior:

- group by SQLSTATE when available, then normalized error text, app, user, and database
- rank by frequency and recency inside the selected window
- preserve representative evidence
- emit follow-up SQL that helps inspect live blockers, active sessions, or related catalog state

### 3. Temp-File Triage

This is the most valuable PostgreSQL-specific third workflow for V1.

Why it is in scope:

- temp files are repeatedly cited as high-signal in pgBadger usage
- they are operationally actionable
- they extend the CLI beyond generic query and error summaries without requiring full lock or plan correlation

V1 behavior:

- surface the top temp-file producers by query family, application, user, or database
- rank by bytes written and event frequency
- preserve evidence pointers to the underlying log records
- emit follow-up SQL for sort, work_mem, and workload inspection

### Shared V1 Output Contract

All three workflows should share the same investigation-oriented output model:

- ranked compact findings
- stable finding identifiers
- evidence handles back to source logs
- deterministic sort order
- optional machine-readable output suited for shell pipelines and LLM use
- suggested next-step SQL

## Consequences

### Positive

- V1 stays aligned with the repo's investigation-first thesis.
- The initial event model only needs to support the workflows with the strongest evidence and best implementation fit.
- The product surface mirrors real pgBadger usage without trying to clone pgBadger.
- The chosen workflows cover three distinct operator loops: performance, failures, and resource pressure.

### Negative

- "New slow queries" diffing is explicitly deferred even though it remains a flagship direction.
- Lock waits, vacuum or analyze triage, PgBouncer correlation, and `auto_explain`-driven workflows remain out of V1.
- Some users will expect broader pgBadger-style reporting than V1 intentionally provides.

### Explicit V1 Deferrals

The following are not V1 workflows:

- baseline versus target slow-query diffing
- vacuum or analyze triage
- lock-wait triage
- PgBouncer correlation
- full `auto_explain` plan correlation
- broad HTML or dashboard reporting

## Alternatives Considered

- Start with two-window "new slow queries" diffing as the first workflow.
- Include vacuum or analyze triage instead of error triage.
- Include lock-wait triage instead of temp-file triage.
- Pursue a wider pgBadger-style report surface for V1.
