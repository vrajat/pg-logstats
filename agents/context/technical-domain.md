<!-- Context: project-intelligence/technical | Priority: critical | Version: 1.0 | Updated: 2026-04-18 -->
# Technical Domain

**Concept**: `pg-logstats` is a Rust-first PostgreSQL log analysis project being repositioned as a PostgreSQL-native observability toolkit.

## Key Points

- Primary shape today is a CLI with a supporting Rust library.
- Current live code parses PostgreSQL stderr logs and produces text or JSON summaries.
- Query normalization uses `sqlparser`.
- The intended direction is structured log ingestion plus correlation with PostgreSQL-native metadata.
- Some repo docs still reflect the older framing as an AI-coding experiment; verify claims against current code and `engg/` docs.

## Primary Stack

| Layer | Technology | Notes |
|-------|-----------|-------|
| Language | Rust | Single-crate project today |
| CLI | `clap` | Entry point in `src/main.rs` |
| Parsing | `regex`, `chrono`, `sqlparser` | Current stderr parser plus SQL normalization |
| Output | text, JSON | Formatters in `src/output/` |
| Testing | Rust unit and integration tests | `tests/unit/*.rs`, `tests/integration_tests.rs` |

## Current Codebase References

- `src/main.rs` - CLI orchestration and file discovery
- `src/parsers/stderr.rs` - current stderr parser
- `src/analytics/queries.rs` - query analysis and aggregation
- `src/analytics/timing.rs` - timing analysis
- `src/output/json.rs` - structured output
- `engg/design/release-and-usability-plan.md` - current release/usability handoff

## Directional Guardrails

- Prefer PostgreSQL-specific semantics over generic logging abstractions.
- Treat `jsonlog`, `csvlog`, `queryid`, `pg_stat_statements`, and `auto_explain` as the future center of gravity.
- Avoid feature sprawl that only recreates `pgBadger` breadth without a stronger thesis.
