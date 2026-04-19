<!-- Context: project-intelligence/living | Priority: high | Updated: 2026-04-18 -->
# Living Notes

## Current Observations

- The README previously positioned the repo mainly as an AI-assisted learning project; it is now being redirected toward product clarity.
- The live implementation is stronger than the positioning, but still limited to stderr parsing.
- `docs/API.md` likely drifts from the live code in a few method names and structure details; treat source as authoritative.
- The CLI includes some flags inspired by broader log tools, but the implementation surface is narrower than the option list suggests.

## Near-Term Priorities

1. Add a normalized event model that can support stderr, `csvlog`, and `jsonlog`.
2. Improve statement plus duration correlation so analytics are not based on loosely related lines.
3. Decide how `queryid` enters the event model and export schema.
4. Design the first structured export path before building richer presentation layers.

## Validation Notes

- Cargo-based validation may require access to the cargo registry cache outside the writable sandbox.
- If tests fail due to environment restrictions, report that separately from code-level failures.
