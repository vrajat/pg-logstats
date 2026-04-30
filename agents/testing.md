# Testing Guide

Choose the smallest validation set that proves the change.

## Default Commands

- `cargo test`
- `cargo test --test integration_tests`

## Parser Changes

When touching log parsing:

- add or update fixtures in `tests/fixtures/cli/`
- run unit tests for parser coverage
- run integration tests that exercise the CLI path

## Output Changes

When touching JSON or text formatting:

- run output-focused unit tests
- confirm the CLI still produces expected top-level sections

## Analytics Changes

When changing query grouping, slow-query logic, or timing:

- run analytics unit tests
- add a realistic integration sample if semantics changed

## Environment Notes

If validation is blocked by sandbox restrictions or missing dependencies, report:

- the command you attempted
- the exact blocker
- whether the blocker is environmental or code-related
