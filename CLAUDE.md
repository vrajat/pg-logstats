# Claude Development Agent

This file configures Claude when working with this repository.

## Load Order

1. `AGENTS.md`
2. `agents/README.md`
3. `agents/process.md`
4. `agents/context/technical-domain.md`
5. `agents/context/living-notes.md`

## Role Selection

- Planner: use for non-trivial design or roadmap work
- Builder: use for implementation tasks
- Tester: use when validating behavior or expanding coverage
- Reviewer: use for code review and regression hunting

## Claude-Specific Notes

- Treat this file as a thin wrapper, not the main instruction body.
- If repo docs conflict with current code, prefer the live code plus `engg/` direction docs.
- In reviews, cite concrete file references and focus on correctness, regressions, and missing tests.
