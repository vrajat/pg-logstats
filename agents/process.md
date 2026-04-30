# Project Process

Shared development process and conventions for `pg-logstats`.

## Usage

- Read `README.md`, relevant `engg/` docs, and the files you plan to change before editing.
- For non-trivial work, confirm the change aligns with `engg/PHILOSOPHY.md` and the implementation plan.
- Keep changes focused. Do not mix unrelated cleanup into parser, analytics, or output work.
- If docs drift from the code, update them or call out the mismatch explicitly.

## Project-Specific Guidance

- Prefer PostgreSQL-specific functionality over generic log-tool features.
- Do not copy `pgBadger` options or report sections without a clear product reason.
- Favor structured event models and reusable output schemas over one-off formatting logic.
- When changing parser behavior, add or update fixtures in `demo/logs/` or tests.
- When changing JSON output, update tests and any docs that describe the schema.

## Related Files

- `engg/PHILOSOPHY.md`
- `engg/design/implementation-plan.md`
- `engg/adr/0001-ripgrep-inspired-cli-philosophy.md`
- `engg/adr/0002-select-v1-investigation-workflows.md`
- `engg/investigations/pg-loggrep-llm-direction.md`
- `agents/context/technical-domain.md`
- `agents/context/living-notes.md`
- `agents/testing.md`

## Validation Expectations

When the environment allows it, prefer:

- `cargo fmt`
- `cargo test`
- `cargo clippy`

If sandbox or dependency constraints block validation, say so clearly and include the concrete failure mode.
