# Design: Release And Usability Plan

Status: proposed  
Date: 2026-04-24

## Overview

This note is a handoff plan for the next `pg-logstats` work slice.

The goal is not to expand product scope. The goal is to make the current
investigation-first CLI easier to trust, easier to install, easier to demo, and
better protected by tests.

The codebase has already landed most of the architectural shift:

- normalized events exist
- correlation exists
- structured findings exist
- the CLI already exposes `top query-families`, `slow-queries diff`, and `suggest-sql`

That means the next slice should focus on software engineering quality and
usability rather than another round of internal redesign.

## Is A Full Design Doc Required?

No.

The architecture is already far enough along that a full design doc would mostly
repeat code that exists. This note is intended to be sufficient for a handoff to
another medium-capability model or engineer.

The only decisions that need to be locked for this slice are:

1. CI should follow the canonical local validation path.
2. Installation should follow `crates.io` first, then Homebrew formula.
3. The default demo should work without PostgreSQL; the Docker demo becomes
   optional and secondary.

## Goals

- Make the checked-in examples and docs match the shipped CLI.
- Make the default demo path work without running PostgreSQL.
- Add CI structure that reflects the local validation contract.
- Improve release readiness for publishing to `crates.io`.
- Expand tests with a small borrowed corpus from `pgbadger`, adapted to
  `pg-logstats` workflows and parser behavior.

## Non-Goals

- Full `pgBadger` feature parity
- HTML reports
- Incremental mode
- Binary output
- Adding many new workflows in this slice
- Broad multiformat support unless the parser work is explicitly in scope

## Current State

The main gaps are around product truthfulness and release hygiene, not basic
code correctness.

Observed problems:

- checked-in sample data does not fully align with the current parser shape
- some CLI help text still reflects inherited `pgBadger` wording
- README, examples, and demo docs describe commands and flags that do not exist
- demo scripts still call older CLI shapes
- some internal project references point at `engg/design/product-requirements.md`,
  which is currently missing
- installation and release metadata are not yet ready for public distribution

## Guiding Decisions

### 1. Buildkite Follows The Local Validation Contract

Use Buildkite for CI, and make it call the same local validation entry points
contributors use.

The target shape should emulate the infrastructure approach in `~/code/pgqrs`:

- keep a checked-in `.buildkite/pipeline.yml`
- keep the pipeline thin by delegating to local task-runner commands
- separate bootstrap and environment setup from the validation commands
- avoid CI-only command definitions that diverge from local usage

Reasoning:

- it keeps release automation separate from day-to-day validation
- it avoids CI-only behavior
- it keeps `pg-logstats` aligned with the existing infra direction used in
  `pgqrs`
- it makes Buildkite the canonical CI surface instead of an interchangeable
  implementation detail

### 2. `crates.io` Before Homebrew

Publish to `crates.io` first, then add a Homebrew formula.

Do not use Homebrew cask for the first release.

Reasoning:

- `pg-logstats` is a CLI binary, which fits formula distribution better than cask
- `cargo install pg-logstats` is the lowest-friction first install path for the
  likely early audience
- Homebrew should sit on top of a release process that already produces tagged,
  trustworthy binaries and metadata

### 3. Fixture Demo First

The default demo path should run entirely on checked-in fixture data.

Reasoning:

- it is faster to demo
- it is easier to trust in CI
- it avoids requiring PostgreSQL just to prove the core CLI experience

The Docker/PostgreSQL demo remains useful, but only as an optional second-tier
demo after the fixture demo is stable.

## Implementation Plan

### Phase 1: Fix The Shipped Happy Path

Goal: make the checked-in examples behave like product documentation, not just
sample files.

Tasks:

- align checked-in sample logs with the parser currently exercised in tests
- make sure the primary README commands work against checked-in fixtures
- clean up stale CLI help text inherited from report-oriented tooling
- prefer duplicating high-frequency UX flags such as `--quiet` on workflow
  commands if practical; if not, document global flag placement explicitly

Expected outcome:

- one working parser-compatible sample corpus
- one working command sequence for `top query-families`
- one working command sequence for `slow-queries diff`
- one working command sequence for `suggest-sql`

### Phase 2: Rewrite User-Facing Docs Around Current Truth

Goal: make `README.md` the canonical product source.

Tasks:

- rewrite the README around the three supported workflows
- explicitly document the currently supported log format(s)
- remove references to unsupported flags or outputs from:
  - `examples/README.md`
  - `demo/README.md`
  - `demo/docker/README.md`
  - any example recipes that still use stale flags
- fix or replace internal references that still point at the missing
  `engg/design/product-requirements.md`

Rules for this phase:

- source and tests win over stale docs
- do not document features that are not in the CLI
- do not preserve old docs "just in case"

### Phase 3: Add An Offline Demo

Goal: make demoing possible without PostgreSQL.

Tasks:

- add one fixture-based demo script under `demo/`
- make it show:
  - `top query-families`
  - `slow-queries diff`
  - `suggest-sql`
- make the script produce stable output suitable for manual demo and CI smoke use

Secondary tasks:

- reduce the Docker demo to an optional advanced path
- update or remove Docker scripts that still call obsolete CLI arguments

### Phase 4: Add Local Validation Entry Points

Goal: stop CI and docs from drifting away from local developer usage.

Tasks:

- add a small local task runner, either `Makefile` or `justfile`
- define canonical commands for:
  - formatting
  - tests
  - clippy
  - demo smoke
  - package smoke

Suggested targets:

- `fmt`
- `test`
- `clippy`
- `demo-smoke`
- `package-smoke`
- `check`

`check` should be the command both humans and CI can trust.

### Phase 5: Align CI With The Local Contract

Goal: make CI reflect the authoritative local validation path.

Tasks:

- add or update `.buildkite/pipeline.yml`
- add steps for:
  - format
  - tests
  - clippy
  - demo smoke
  - package smoke
- keep the Buildkite configuration thin by having it call the local task runner
- mirror the `pgqrs` split between bootstrap/setup steps and validation steps
  where that structure makes sense for `pg-logstats`

Constraint:

- Buildkite should call the local task runner, not re-specify all raw cargo
  commands inline

### Phase 6: Release Readiness

Goal: make the project publishable and installable.

Tasks:

- clean up `Cargo.toml` metadata
- verify package contents with `cargo package`
- verify installability with local `cargo install --path .`
- add a tag-based release workflow for crates publication
- add follow-on Homebrew formula work after crate publication is stable

Install order:

1. `cargo install` from source for local development
2. publish to `crates.io`
3. add Homebrew formula

## Testing Plan

The current suite is useful, but it over-indexes on internals and does not
protect the real user paths strongly enough.

This slice should add tests in four groups.

### 1. Product Smoke Tests

- README command smoke tests
- checked-in fixture smoke tests
- offline demo script smoke tests
- CLI help text regression tests

### 2. Parser Regression Tests

- multiline statements
- bind parameters
- range parameters
- unicode payloads
- statement classification
- mixed valid and invalid lines

### 3. Findings And Output Stability Tests

- one stable text-output golden test
- one stable JSON findings golden test
- one `suggest-sql` smoke test using checked-in findings JSON

### 4. Release Smoke Tests

- `cargo package`
- local install smoke via `cargo install --path . --root <tempdir>`
- binary help/version smoke

## Borrowed Tests And Test Data From `pgbadger`

Borrow fixture intent, not report-oriented behavior.

Do not copy:

- HTML report assertions
- binary output assertions
- incremental mode assertions
- multi-output report assertions
- report-per-database assertions

These are mismatched with `pg-logstats`.

For this slice, keep the import narrow: prefer 2 to 3 high-signal fixtures that
protect the current parser and workflow surface, rather than a broad corpus
import.

### Borrow Now

#### `t/fixtures/stmt_type.log`

Use for:

- parser coverage
- statement classification coverage
- normalization coverage

Adaptation:

- trim to a smaller stderr-only fixture
- assert `pg-logstats` query-family grouping or normalized SQL behavior

Source test:

- `t/02_basics.t`

#### `t/fixtures/multiline_param.log`

Use for:

- multiline statement parsing
- multiline bind parameter handling

Adaptation:

- convert into a focused parser fixture
- assert that multiline query text is reconstructed correctly enough for current
  normalization and grouping

Source test:

- `t/04_advanced.t`

#### `t/fixtures/postgresql_param_range.log`

Use for:

- range-literal parsing
- parameterized range predicate normalization

Adaptation:

- trim to the minimal lines needed
- assert normalized SQL stability

Source test:

- `t/04_advanced.t`

#### `t/fixtures/tempfile_only.log.gz`

Use for:

- future temp-file workflow
- future demo material

Adaptation:

- keep as a trimmed plain-text source fixture if possible
- do not build temp-file product surface in this slice unless explicitly chosen

Source test:

- `t/04_advanced.t`

#### `t/fixtures/queryid.log.gz`

Use for:

- future `queryid` event model coverage

Adaptation:

- reserve or import into a `future` fixture area with provenance notes

### Borrow Later

#### `t/fixtures/pg_vacuums.log.gz`
#### `t/fixtures/pg_vacuums.json.gz`

Use later for:

- autovacuum and analyze workflows
- stderr/jsonlog parity work

Source test:

- `t/03_consistency.t`

#### `t/fixtures/cloudsql.log.gz`
#### `t/fixtures/cnpg.log.gz`
#### `t/fixtures/rds.log.bz2`
#### `t/fixtures/logplex.gz`

Use later for:

- multiformat parser acceptance coverage

These should only be imported if parser support is actually added.

### Usually Skip

#### `t/fixtures/anonymize.log`

Only import if anonymization becomes a real product feature.

#### `t/fixtures/weeknumber.log`

Only import if time-window or calendar bucketing becomes a real CLI feature.

## Suggested New Repo Layout For Borrowed Fixtures

Suggested structure:

```text
tests/
  fixtures/
    borrowed/
      README.md
      stmt_type_minimal.log
      multiline_param_minimal.log
      range_param_minimal.log
      tempfile_minimal.log
```

The `README.md` in that directory should record:

- original `pgbadger` fixture name
- original `pgbadger` test file
- what was trimmed or transformed
- what `pg-logstats` behavior the adapted fixture now protects

## Suggested Execution Order

1. fix checked-in fixture compatibility and CLI wording
2. rewrite README and example docs around the current CLI
3. add offline demo script and smoke test
4. import 2 to 3 trimmed high-signal `pgbadger`-derived parser fixtures and
   regression tests
5. add local task runner
6. align CI with the canonical validation path
7. finish crate packaging and release prep
8. add Homebrew formula after crate publication is credible

## Definition Of Done

- checked-in README commands work against checked-in fixtures
- the default demo works without PostgreSQL
- stale flags and outputs are removed from docs and scripts
- adapted `pgbadger` fixtures are imported with provenance notes
- CI runs the canonical validation path
- package and install smoke checks pass locally and in CI

## Open Questions

1. Should temp-file coverage remain test-only in this slice, or should it pull
   forward the `temp-files` workflow?
2. Should borrowed fixtures remain compressed when compression is not itself
   under test?
3. Should the release workflow publish binaries immediately, or stop at crate
   publication for the first release?
