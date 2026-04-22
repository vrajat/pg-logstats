# Architecture Decision Records

ADRs capture durable project decisions.

## When to Add an ADR

Add an ADR when a change affects:

- project scope or positioning
- core data model
- parser architecture
- output schema compatibility
- major workflow or process changes

## Format

- File name: `NNNN-short-kebab-case-title.md`
- Use the template in [`template.md`](template.md)
- ADRs should remain append-only after acceptance; supersede them with new ADRs instead of rewriting history

## Current ADRs

- [0001 - Adopt a ripgrep-inspired CLI philosophy for pg-logstats](0001-ripgrep-inspired-cli-philosophy.md)
- [0002 - Select three investigation workflows for V1](0002-select-v1-investigation-workflows.md)
