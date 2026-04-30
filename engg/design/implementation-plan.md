# Implementation Plan: Investigation-First Direction

Status: draft  
Date: 2026-04-19

## Overview

This plan translates the new product direction into an execution sequence for the current `pg-logstats` codebase.

The repo today is a Rust CLI that parses PostgreSQL `stderr` logs and emits broad text or JSON summaries. The target product is narrower and sharper: **`pg-logstats` as a PostgreSQL-native investigation CLI** that produces compact, structured findings and helps the operator decide what to inspect next.

The implementation plan therefore focuses on three transitions:

1. from raw log-entry summaries to a reusable structured event model
2. from loosely related query analytics to correlation-aware investigation workflows
3. from generic report output to compact, evidence-preserving findings

## Product Thesis

The tool should sit between PostgreSQL logs and live database inspection.

- `pg-logstats` narrows and ranks historical evidence
- `psql` remains the tool for deeper live interrogation
- `pgBadger` remains the broader reporting tool
- external agents or scripts may call `pg-logstats` early in a triage loop, but orchestration remains outside the CLI

The first implementation slices should reinforce that thesis rather than chase report breadth.

## Goals

- Establish an internal event model that can support `stderr`, `csvlog`, and `jsonlog`.
- Improve correlation between statements, durations, errors, and future companion signals.
- Introduce workflow-oriented commands that emit ranked findings rather than full summaries.
- Make structured output the durable contract for humans, shell tools, and LLM agents.
- Preserve a practical migration path from the current `pg-logstats` CLI.

## Non-Goals

- Full `pgBadger` feature parity
- Dashboard-first or HTML-report-first work
- Generic non-PostgreSQL log analysis
- Automatic SQL tuning or indexing advice in the first implementation
- Broad companion-source ingestion before the core event model and finding schema are stable
- Built-in chat, agent orchestration, or autonomous investigation flows

## Current State

The current codebase is a workable starting point, but it is still aligned to the older summary-oriented CLI.

- `src/parsers/stderr.rs` produces `LogEntry` records for `stderr` logs only.
- `src/lib.rs` models log entries directly rather than a normalized cross-format event layer.
- `src/analytics/queries.rs` aggregates queries mostly from statement entries and uses weak duration linkage.
- `src/output/json.rs` emits report-style summaries, not finding-oriented schemas.
- `src/main.rs` exposes a single analysis command shape with options inherited partly from older tooling.

The design implication is that the first meaningful refactor is internal, not cosmetic.

## Guiding Principles

- Keep the CLI small and workflow-oriented.
- Prefer event-model work over presentation work.
- Prefer explicit and deterministic baseline/target inputs before clever heuristics.
- Use stable identifiers and evidence references everywhere they matter.
- Treat machine-readable findings as a public contract for shell tools and external AI workflows.
- Preserve backwards compatibility only when it does not fight the new product direction.

## Proposed Architecture Direction

### 1. Normalized Event Layer

Introduce a new event model above raw parser output.

The model should capture:

- timestamp
- source reference
- process or session identity
- database, user, app, and client metadata
- event kind
- normalized statement data
- duration data
- error data including SQLSTATE when available
- optional correlation keys such as `queryid`

This layer should become the shared input to workflows and output schemas.

### 2. Correlation Layer

Add a correlation stage that can associate related PostgreSQL log records into a more useful investigative unit.

Initial focus:

- statement plus duration pairing for `stderr`
- query-family identity generation
- evidence references back to source entries

Later extensions:

- `jsonlog` and `csvlog`
- `auto_explain`
- `pg_stat_statements` snapshots

### 3. Finding Layer

Add a finding model for ranked investigative output.

A finding should include:

- stable `finding_id`
- workflow-specific kind
- compact title and reason
- compact machine-readable reason codes for reproducible downstream use
- score or rank inputs
- grouped dimensions such as query family, SQLSTATE, app, user, or database
- summary metrics
- evidence pointers
- optional confidence or suppression notes when ranking depends on heuristics
- suggested follow-up SQL when relevant

### 4. Command Layer

Move from one generic analysis command toward explicit workflow commands.

Early target shape:

- `pg-logstats top query-families`
- `pg-logstats errors`
- `pg-logstats evidence <finding-id>`
- `pg-logstats slow-queries diff`

The command surface should stay smaller than the internal capability surface.

## Implementation Phases

### Phase 0: Product and Naming Stabilization

Goal: reduce ambiguity before deeper code changes.

Deliverables:

- document the intended product direction in stable repo docs
- confirm that `pg-logstats` remains the primary product and binary name
- align README and design indexes to the new thesis
- treat missing docs referenced by process files as explicit follow-up items, not silent assumptions

Exit criteria:

- implementation plan approved
- naming posture decided

### Phase 1: Internal Event Model

Goal: decouple downstream workflows from the current `LogEntry` shape.

Deliverables:

- add a normalized event module with event kinds and source references
- map current `stderr` parser output into normalized events
- preserve enough metadata for future `jsonlog` and `csvlog`
- define stable event and source-reference structures for tests

Code impact:

- new module under `src/` for normalized events
- parser pipeline changes in `src/parsers/` and `src/lib.rs`
- targeted test fixtures covering event conversion

Exit criteria:

- current `stderr` parsing still works
- analytics can operate on normalized events instead of raw log entries

### Phase 2: Correlation and Query Family Identity

Goal: fix the biggest analytical weakness in the current implementation.

Deliverables:

- pair statements with durations using process and temporal context
- define query-family identity from normalized SQL plus available metadata
- attach evidence references to correlated records
- make identity generation deterministic and testable

Why this phase matters:

Without better correlation, higher-level workflows will rank misleading data.

Exit criteria:

- slow-query and frequency analytics use correlated executions rather than statement-only approximations
- fixtures cover ambiguous and non-ambiguous pairing cases

### Phase 3: Structured Finding Schema

Goal: make machine-readable findings the primary product asset.

Deliverables:

- define a versioned finding schema
- implement a compact JSON formatter for findings
- keep text output as a thinner view over the same data
- include evidence handles and ranking explanations in the schema

Suggested first schema families:

- query-family finding
- error-class finding
- evidence record

Exit criteria:

- finding JSON can support both CLI use and downstream tooling
- output tests assert schema stability

### Phase 4: First Workflow Command

Goal: ship one high-value workflow on top of the new layers.

Recommended first slice:

- `top slow query families in one window`

Rationale:

- reuses the new event and correlation work
- easier to test than baseline novelty logic
- aligns with the long-term slow-query direction

Deliverables:

- explicit command for single-window top slow query families
- ranked compact output
- evidence lookup path for surfaced findings

Alternative if simpler first value is preferred:

- `new error classes`

This remains the easiest workflow to build and verify, but it exercises less of the query-correlation architecture.

Exit criteria:

- one workflow feels investigation-first rather than report-first
- fixtures and integration tests prove ranking determinism

### Phase 5: Baseline Versus Target Diffing

Goal: reach the flagship “new slow queries” direction without overcomplicating earlier phases.

Deliverables:

- explicit baseline and target inputs
- simple eligibility thresholds
- deterministic regression scoring
- reasons such as “absent in baseline”, “p95 regressed”, or “runtime contribution increased”

Design constraint:

Prefer explicit baseline and target windows or files before introducing rolling adaptive baselines.

Exit criteria:

- `slow-queries diff` produces explainable results on small fixtures
- the ranking logic is simple enough to document and debug

### Phase 6: Companion Correlation and Suggested SQL

Goal: complete the handoff from logs to live inspection.

Deliverables:

- `suggest-sql` support for relevant findings
- optional `pg_stat_statements` enrichment
- optional `auto_explain` enrichment
- stable hooks for future evidence drilldowns

Exit criteria:

- the tool can surface a finding and suggest the next concrete SQL to run

## Command Evolution Plan

The repo currently exposes `pg-logstats` with a summary-oriented CLI. The transition should preserve that binary name while the command model becomes more workflow-oriented.

Recommended sequence:

1. keep the current top-level CLI working during Phases 1 through 3
2. introduce new workflow subcommands under `pg-logstats`
3. retire or demote the older summary-oriented flow only after the new workflows are clearly better
4. use docs and help text to explain the investigation-first shift instead of renaming the binary

## Data Model Priorities

These fields should be treated as first-class in the new event and finding models:

- timestamp
- source file plus line or byte reference
- process or session identity
- database
- user
- application name
- normalized SQL
- `queryid` when available
- duration
- SQLSTATE
- finding rank and reason
- reason code
- suppression or filter notes
- suggested follow-up SQL

## Validation Strategy

### Unit Tests

Add focused tests for:

- event conversion from `stderr`
- statement and duration correlation
- query-family identity generation
- ranking heuristics
- finding-schema serialization

### Integration Tests

Use small deterministic fixtures for:

- one-window top slow query families
- two-window slow-query diff
- error grouping
- evidence lookup

### Fixture Strategy

Prefer compact fixtures in `tests/fixtures/cli/` or focused unit fixtures that cover:

- stable fast queries
- stable slow queries
- newly slow queries
- noisy low-count outliers
- correlated and uncorrelated duration lines

## Risks

- Correlation for `stderr` may stay heuristic in some edge cases.
- Renaming the binary too early could create churn before the new workflows are compelling.
- If the event model is too tied to current `stderr` details, `jsonlog` and `csvlog` support will be harder later.
- If the first workflow is over-ambitious, the project may stall before shipping a sharp CLI loop.

## Recommended Immediate Next Steps

1. Approve this phase structure and the first workflow choice.
2. Confirm the naming posture and remove remaining rename assumptions from docs and CLI planning.
3. Hand off Phases 1 through 3 plus single-window `top query-families` as the initial implementation scope.
4. Implement Phase 1 by introducing the normalized event model without changing user-facing behavior yet.
5. Add deterministic fixtures that will also support the later correlation and finding tests.

## Open Questions

- Is the first shipped workflow `top slow query families` or `new error classes`?
- Should `queryid` be optional metadata in Phase 1, or a required design axis from the start?
- Should evidence references use line numbers, byte offsets, or a source-agnostic handle abstraction?
- Should any subcommand naming borrow search-oriented language, or should the product stay fully analysis-oriented in its command vocabulary?
