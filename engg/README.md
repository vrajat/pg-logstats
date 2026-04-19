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
2. [`adr/0001-product-direction-modern-postgres-observability.md`](adr/0001-product-direction-modern-postgres-observability.md)
3. [`design/product-requirements.md`](design/product-requirements.md)
4. [`design/implementation-plan.md`](design/implementation-plan.md)
5. [`investigations/postgres-logging-landscape.md`](investigations/postgres-logging-landscape.md)

## Contribution Notes

- Use ADRs for durable scope and architecture decisions.
- Use design docs for plans that will evolve over time.
- Prefer concise documents with explicit goals, non-goals, alternatives, and open questions.
