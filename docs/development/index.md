---
title: pg-logstats Development Guide
description: Build, test, and document pg-logstats locally with Zensical, Rust tooling, and the repo conventions used for PostgreSQL triage workflows.
schema:
  "@context": "https://schema.org"
  "@type": "TechArticle"
  headline: "pg-logstats Development Guide"
  description: "Contributor setup and local development tasks for pg-logstats."
  url: "https://pg-logstats.vrajat.com/development/"
---

# Development

This section is for contributors working on the `pg-logstats` repository.

## Top-Level Layout

- `src/`: Rust library and binary implementation
- `docs/`: public documentation and Zensical source
- `tests/`: integration tests and test fixtures
- `agents/`: AI agent harnesses and playbooks
- `engg/`: engineering design documents

## Local Tasks

Use `mise` for tool versions and `make` for tasks:

```bash
# Install toolchains (Rust, Python, UV)
mise install

# Run checks (format, clippy, package-smoke)
mise exec -- make check

# Run test suite
mise exec -- make test

# Build documentation site
mise exec -- make docs-build
```

Use `make docs` when you want to run the local Zensical development server:

```bash
mise exec -- make docs
```

## Documentation Rule

Public docs should help a user or operator use `pg-logstats` or develop against its library. Internal design rationale should be placed in `engg/` or in `development/architecture.md`.

When a CLI command or public API changes, update the matching docs page in the same commit.
