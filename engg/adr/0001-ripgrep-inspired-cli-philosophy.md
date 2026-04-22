# ADR 0001: Adopt a ripgrep-inspired CLI philosophy for pg-logstats

- Status: Accepted
- Date: 2026-04-19

## Context

`pg-logstats` is moving toward a new product direction centered on PostgreSQL log investigation.

The project needs a durable product philosophy that explains why a new CLI should exist alongside:

- `psql`, which is already powerful for live-state inspection
- `pgBadger`, which is already powerful for broad PostgreSQL log reporting

The risk is drifting into one of two weak positions:

1. a partial `pgBadger` clone with less breadth
2. a vague “AI for Postgres” wrapper without a strong command-line core

The ripgrep design lesson is relevant here: a command-line tool becomes important when it combines:

- simple mental model
- strong utility in the default workflow
- performance good enough for constant use

Primary reference:

- Andrew Gallant, "ripgrep is faster than {grep, ag, git grep, ucg, pt, sift}" — <https://burntsushi.net/ripgrep/>

That philosophy aligns with the needs of both human investigators and external LLM-driven workflows, where speed and token efficiency matter.

The intent is not to embed AI workflow orchestration inside `pg-logstats`. The CLI should instead act as a fast, trustworthy source of grounded PostgreSQL log evidence that other tools can consume.

## Decision

We will build `pg-logstats` as a **CLI-first PostgreSQL log investigation tool inspired by the product philosophy of ripgrep**.

This means:

1. optimize for simple, explainable investigation workflows rather than report breadth
2. treat performance as a first-class product feature
3. prefer compact, structured findings over large report outputs
4. use smart defaults with explicit escape hatches
5. keep the tool PostgreSQL-specific rather than becoming a generic log search engine
6. design the CLI to compose naturally with `psql`, shell tools, and external LLM skills or agents

In practical terms, the primary command surface should focus on workflows like:

- top query families
- top errors
- evidence lookup
- baseline versus target diffing
- suggested next-step SQL

## Consequences

### Positive

- The project has a clearer reason to exist next to `psql` and `pgBadger`.
- The command-line interface can stay small and coherent longer.
- Performance, token efficiency, and composability become explicit design goals.
- The tool can serve both human CLI workflows and LLM-assisted triage without becoming unnatural for either.
- The CLI stays focused on evidence production rather than embedded AI orchestration.

### Negative

- The project will intentionally lag behind `pgBadger` on broad reporting features.
- Some familiar dashboard or HTML features should be delayed even if they are tempting.
- The scope boundary must be defended continuously to avoid becoming a generic log tool.

## Alternatives Considered

### 1. Build a broad pgBadger-style report generator first

Rejected. That competes on breadth where `pgBadger` is already strong and does not create a crisp product wedge.

### 2. Build an LLM skill around `psql` alone

Rejected. `psql` is excellent for live-state queries, but it does not solve structured ingestion, ranking, correlation, or evidence-preserving analysis of PostgreSQL logs.

### 3. Build a dashboard-first product

Rejected for now. Presentation is less important than a fast, composable, investigation-grade CLI core.
