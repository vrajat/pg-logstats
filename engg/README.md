# Engineering Documentation

This directory contains internal planning and engineering documentation for `pg-logstats`.

## Structure

### `/adr/` - Architecture Decision Records
Immutable records of significant decisions about project scope, architecture, and workflow.

### `/design/` - Design and Product Planning
Living design documents for project direction, product requirements, and implementation plans.

### `/investigations/` - Research and Spikes
Research notes that justify product direction, parser strategy, benchmarks, and technical bets.

### `/processes/` - Engineering Process
Project-specific development and documentation processes.

### `/reviews/` - Review Artifacts
Notes from deeper design or code reviews when a change deserves durable documentation.

## Current Recommended Reading Order

1. [`PHILOSOPHY.md`](PHILOSOPHY.md)
2. [`adr/0001-ripgrep-inspired-cli-philosophy.md`](adr/0001-ripgrep-inspired-cli-philosophy.md)
3. [`investigations/pg-loggrep-llm-direction.md`](investigations/pg-loggrep-llm-direction.md)

## Contribution Notes

- Use ADRs for durable scope and architecture decisions.
- Use design docs for plans that will evolve over time.
- Prefer concise documents with explicit goals, non-goals, alternatives, and open questions.
