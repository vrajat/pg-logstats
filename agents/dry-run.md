# Dry-Run Process

Use this process when validating an investigation workflow as a product, not
just as a unit of code.

The goal is to simulate an agent using `pg-logstats` step by step, inspect the
actual transcript and persisted artifacts, and convert every gap into code,
tests, or docs until the workflow is credible.

## When To Use This

Use a dry-run when:

- a workflow is new or recently reshaped
- the CLI or JSON contract is changing
- a workflow needs agent-first usability review
- docs claim a workflow exists but the end-to-end transcript has not been
  checked recently

This is the preferred process for beta-shaping AX.

## Two-Agent Setup

This process should normally be run with two collaborating agents.

Use one agent for the operator transcript and one agent for product review.

Do not merge both roles into one agent unless the environment cannot support
sub-agents. The point is to preserve the tension between:

- what an agent would actually do next
- whether that behavior is actually acceptable

## Roles

### Role 1: Agent Operator

This agent plays the investigation agent.

Responsibilities:

- read the current output only
- choose the next command from the product surface
- avoid using hidden repo knowledge to skip steps
- paste the exact command transcript, including mistakes

This agent should act like a real agent harness, not like a developer who
already knows how the code works.

### Role 2: Product Reviewer

This agent reviews the transcript and the persisted artifacts.

Responsibilities:

- judge whether the previous step was correct
- compare behavior to the intended runbook
- decide whether the problem is in:
  - code
  - tests
  - docs
  - product shape
- implement fixes before continuing when the issue is structural

This agent should not excuse awkward or misleading behavior just because the code
is internally consistent.

## Coordination Contract

The two agents should collaborate like this:

1. Reviewer defines the scenario, constraints, and success bar.
2. Operator runs one command or proposes the exact next command.
3. Reviewer inspects:
   - transcript
   - persisted artifacts
   - next actions
   - docs implications
4. Reviewer either:
   - approves the next step
   - or stops the run and fixes the product
5. Repeat until the workflow reaches a credible terminal state.

The operator should never silently correct awkward product behavior on behalf of
the tool.

The reviewer should never skip transcript review just because a command
"probably works."

## Core Rule

Do not skip ugly or mistaken steps.

If the operator reaches for the wrong command, that is signal. Fix the product
or document why the behavior is intentional.

The dry-run is not a demo. It is an adversarial transcript review.

## Modes

There are three valid dry-run modes.

### 1. Real Offline

Use real logs and real CLI execution for log-backed workflows.

Use this for:

- `inspect`
- `query-families`
- `errors`
- `temp-files`
- `slow-queries diff`

This is the default mode for workflow validation.

### 2. Fully Synthetic

Pre-seed persisted artifacts and inspect or review them without executing the
corresponding command.

Use this only when the workflow step cannot reasonably be executed yet, or when
the product contract itself is under review.

### 3. Hybrid

Use real execution for log-backed steps and mocked artifacts for live steps.

This is the preferred mode for current `pg-logstats` workflow validation.

Current rule:

- log-backed commands should run for real
- live commands may be mocked

Examples:

- real: `inspect` over a synthetic log file
- real: `query-families` over that log file
- mocked: a live-capable `inspect.json`
- mocked: a `run_sql` report that simulates a bounded `pg_stat_activity` result

## Standard Sequence

For each workflow, the two agents should run this sequence.

1. Reviewer defines the scenario.
2. Reviewer creates or chooses the workspace and inputs.
3. Operator runs the transcript one command at a time.
4. Reviewer checks after each command:
   - stdout or stderr
   - persisted artifacts
   - next action shape
   - whether the next step is obvious
5. Reviewer fixes structural issues immediately.
6. Operator re-runs from the relevant entrypoint.
7. Once the sequence is credible, reviewer converts the scenario into tests and docs.

## Scenario Setup

Every dry-run should have a named workspace under `target/workspaces/`.

Example:

```text
target/workspaces/slow-query-basic/
  README.md
  postgresql.log
  config.toml            # optional
  inspect.json           # optional synthetic artifact
  reports/               # persisted reports or mocked live follow-ups
```

Keep the scenario intentionally small.

For a slow-query scenario, one dominant family plus one or two weaker families
is usually enough.

## Transcript Rules

The operator agent should run commands exactly as an agent would.

Examples:

```bash
pg-logstats query-families --workspace target/workspaces/slow-query-basic target/workspaces/slow-query-basic/postgresql.log
```

If that fails because `inspect.json` is missing, the reviewer should not say
"you should have known to run inspect." The reviewer should judge whether the
product response was clear enough.

The operator agent then continues from the product's next step:

```bash
pg-logstats inspect --workspace target/workspaces/slow-query-basic target/workspaces/slow-query-basic/postgresql.log
```

The exact mistakes are part of the artifact. Keep them.

## Review Checklist Per Step

For each operator command, the reviewer agent checks:

- Was the command shape natural?
- Did the error or success message make the next step obvious?
- Did the product preserve enough context to continue?
- Did persisted artifacts land in the right place?
- Did the output contain internal scaffolding instead of product signal?
- Was the next action:
  - executable now
  - delegated to the operator
  - terminal
  - or a dead-end

## What To Fix Immediately

Stop and fix immediately when you find:

- the wrong entrypoint is too easy to choose
- a follow-up command is not replayable
- the report schema exposes internal scaffolding instead of agent signal
- an output dead-ends where a clear next action should exist
- docs and CLI behavior disagree in a way that misleads the operator

Do not defer these to "later polish." These are core product issues.

## What To Mock

Mock only what cannot yet be run honestly.

Current guidance:

- Do not mock log-backed parsing and ranking if the scenario can be expressed in
  a small fixture log.
- It is acceptable to mock live DB state when the goal is product-shape review.
- When mocking, mirror the real artifact schema exactly.
- Never describe a mocked artifact as the result of a real command.

If a mocked artifact must be used, say so plainly in the scenario README or in
review notes.

## Artifact Standards

A good dry-run should leave behind:

- a small scenario workspace under `target/workspaces/`
- one or more checked-in or reproducible input files
- the final expected command sequence
- one or more persisted report examples
- tests that exercise the stable parts of the flow
- docs that explain the workflow and any operator delegation points

## Converting A Dry-Run Into Tests

Once a scenario is credible, convert the stable pieces into tests.

Priority:

1. Integration tests for command gating and transcript-level behavior
2. Unit tests for extracted decision logic
3. Golden JSON or text tests for stable output contracts

Do not force everything into integration tests if the logic can be extracted and
unit-tested cheaply.

Examples:

- startup gating belongs in integration tests
- `pg_stat_activity` insight derivation belongs in unit tests
- replayable next action hydration can have unit and integration coverage

## Converting A Dry-Run Into Docs

Every successful dry-run should update docs in at least one of these places:

- workflow reference page
- guidance or schema docs
- setup docs if operator delegation changed
- `agents/` process docs if the development process itself improved

Use public docs for product behavior.
Use `agents/` docs for development method.

## Slow Query Example

The slow-query workflow we used is the reference pattern.

### Scenario

- reviewer agent owns scenario setup and code changes
- operator agent owns the transcript and next-command choices
- workspace: `target/workspaces/slow-query-basic`
- real log-backed steps
- mocked live follow-up steps

### Sequence

1. Operator tries the wrong entrypoint:

```bash
pg-logstats query-families --workspace target/workspaces/slow-query-basic target/workspaces/slow-query-basic/postgresql.log
```

2. Product should fail clearly because `inspect.json` is missing.

3. Operator runs:

```bash
pg-logstats inspect --workspace target/workspaces/slow-query-basic target/workspaces/slow-query-basic/postgresql.log
```

4. Reviewer checks:
   - `operating_mode`
   - `failed_checks`
   - replayable `next_actions`

5. Operator runs:

```bash
pg-logstats --workspace target/workspaces/slow-query-basic query-families target/workspaces/slow-query-basic/postgresql.log
```

6. Reviewer checks:
   - findings rank correctly
   - next actions are actionable
   - report IDs and linkage are replayable

7. For live follow-up, use either:
   - a real DB-backed `run-sql`, or
   - a mocked `run_sql` report under `<workspace>/reports/`

8. Reviewer checks whether the live result:
   - surfaces bounded insights when justified
   - or honestly returns no insight

## Output Of A Good Dry-Run

A workflow dry-run is successful when:

- the transcript is credible from the first command onward
- mistakes lead to clear recovery
- reports are replayable and auditable
- next actions feel like product behavior, not rule-engine residue
- the scenario can be handed off to another agent without oral context

## Where This Lives

This process belongs in `agents/` because it is a development method for
building `pg-logstats`, not public operator documentation.
