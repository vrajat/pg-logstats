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
- When changing parser behavior, add or update fixtures in `tests/fixtures/cli/` or parser tests.
- When changing JSON output, update tests and any docs that describe the schema.

## Documentation Style

- Lead with the product, the job it does, and the boundary that makes it worth using.
- Do not open docs by describing the audience in a literal or administrative way.
- Prefer product-led framing over internal framing such as "important boundaries" or process narration.
- Treat public docs as operator-facing product documentation, not as internal notes cleaned up for publication.
- For `pg-logstats`, docs should explain why an expert would install it for agents, how the control path works, and where runbook ends and agent judgement begins.
- Workflow pages should read as audit and runbook references, not as command-by-command CLI tutorials for manual users.

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

- `make fmt`
- `make check`

If sandbox or dependency constraints block validation, say so clearly and include the concrete failure mode.
