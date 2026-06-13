# ADR 0003: Agent-First PostgreSQL Triage Gateway For Internal AI Apps

- Status: Proposed
- Date: 2026-06-13

## Context

`pg-logstats` already has two accepted direction-setting decisions:

- [ADR 0001](0001-ripgrep-inspired-cli-philosophy.md): keep the product
  CLI-first, investigation-oriented, fast, and composable
- [ADR 0002](0002-select-v1-investigation-workflows.md): prioritize
  `top query-families`, `errors`, and `temp-files` as the initial workflows

Those decisions still leave room for a damaging ambiguity:

- is `pg-logstats` meant to be a better human CLI than `pgBadger`?
- is it an agent-friendly helper around direct database access?
- or is it the controlled gateway agents should use to perform PostgreSQL triage?

The clarified product philosophy is:

1. `pg-logstats` is meant primarily for agents
2. giving agents direct database access is a bad default
3. `pg-logstats` should be the gateway through which agents perform database
   triage

This also clarifies the human role.

The human user is not the primary CLI operator. The human is the expert who:

- installs `pg-logstats`
- installs the agent guidance or skill layer
- configures the environment so agent triage is trustworthy
- audits what the agent is allowed to do
- understands the runbook being automated
- takes over directly with tools such as `pgBadger`, `psql`, or deeper SRE
  practice when escalation is required

This is important because `pgBadger` is already the stronger human-facing tool.
`pg-logstats` should not try to beat `pgBadger` at human exploratory analysis.
Its reason to exist is to give agents a safe, auditable, PostgreSQL-specific
triage path.

Another clarified boundary is that every triage workflow is a combination of:

- a runbook
- judgement

Traditional PostgreSQL tooling made the runbook easier for humans while humans
supplied the judgement. In this product, `pg-logstats` packages the runbook and
the agent supplies the judgement at the allowed branch points.

The canonical user story and investigation loop remain the one defined in
`engg/design/internal-ai-app-triage-v1.md`.

## Decision

`pg-logstats` will be built as an **agent-first PostgreSQL triage gateway for
internal AI-built applications**.

The core product promise is:

> install `pg-logstats` and its agent guidance so an agent can perform
> first-pass PostgreSQL triage through a controlled gateway without direct
> database access.

### 1. Primary User Ordering

The primary users, in order, are:

1. a coding agent performing first-pass triage
2. an expert human setting up and auditing that agent workflow
3. an SRE or DBA receiving an escalation with evidence

AX wins over human CLI UX when tradeoffs appear.

The human is still important, but mainly as:

- installer
- operator of the setup
- reviewer of the workflow
- escalation target

Human-driven command discovery is not the primary product job.

### 2. Product Boundary Against pgBadger

`pgBadger` remains the better tool for direct human log investigation.

`pg-logstats` should therefore not position itself as:

- a general human investigation console
- a broad report generator competing on coverage
- a better manual CLI for experts than `pgBadger`

Instead it should position itself as:

- an automation layer for established PostgreSQL triage runbooks
- a compact evidence producer for agents
- a safe bridge from logs to approved diagnostic SQL

### 3. Gateway Boundary

Agents should not receive general direct database access as the default product
model.

`pg-logstats` is the only intended gateway for agent-driven database triage.

For beta, that means:

- the agent invokes `pg-logstats`
- `pg-logstats` decides which workflows are valid
- `pg-logstats` decides which next actions are valid
- `pg-logstats` executes only built-in approved SQL actions
- the agent selects an `action_id`; it does not send arbitrary SQL through the
  product path

The beta product should not expose a general arbitrary-SQL execution mode for
non-development usage.

### 4. Canonical Workflow

The canonical workflow is the user story from
`engg/design/internal-ai-app-triage-v1.md`.

In executable terms, the intended loop is:

1. human installs `pg-logstats`
2. human installs agent guidance with `pg-logstats agent install`
3. agent runs `inspect`
4. `inspect` determines whether the environment is ready for the supported
   workflow
5. if ready, agent runs one triage workflow such as `top query-families`
6. agent chooses one valid `next_action`
7. agent may invoke `run-sql` only by selected built-in `action_id`
8. agent stops or escalates explicitly

This is a bounded investigation loop, not a free-form exploration loop.

### 5. Readiness Contract

For beta, the supported operating posture is:

1. `log_backed`
2. `unready`

`live_only` is not a supported beta success path.

If the environment does not provide the required evidence for a workflow,
`pg-logstats` should stop and say so. It should not promise a degraded version
of the workflow.

Implications:

- no historical ranking claims without logs
- no temp-file triage without the required temp-file logs
- no pretending that weak evidence is sufficient because an agent asked anyway
- `inspect` should make missing prerequisites explicit and actionable

The product should prefer an honest exit over a weak partial success.

### 6. Runbook And Judgement Boundary

Every triage workflow should be documented and implemented as:

- a known PostgreSQL runbook packaged by `pg-logstats`
- plus judgement supplied by the agent at explicit branch points

`pg-logstats` owns:

- the packaged workflow
- the ranking and evidence shape
- the safe next-action graph
- the policy checks on SQL execution
- the stop and escalate boundaries

The agent owns:

- deciding which branch best fits the incident
- deciding whether to stop or escalate sooner than the minimum workflow
- explaining the judgement when several valid branches exist

This boundary must be visible in both the JSON contract and the human-facing
docs.

### 7. Source Of Runbook Truth

`pg-logstats` is not inventing new diagnostic theory.

Its workflows should be derived from recognizable PostgreSQL operational
practice, including sources such as:

- PostgreSQL documentation
- `pgBadger` workflows and reports
- credible PostgreSQL operational write-ups
- bounded prior art for specific diagnostic loops

The product value is automation, packaging, safety, and agent-usable structure,
not novelty of the runbook itself.

This should also shape tests:

- tests should verify the automated runbook behavior
- workflow coverage should be checked against the source runbook
- dry-run investigations should be compared against credible prior art

### 8. Stable Product Interface

The stable contract remains:

- the CLI
- deterministic JSON output
- installed agent guidance or skill wrappers

Codex is the first-class target harness when a tie must be broken, but the
differences between supported coding-agent environments should remain thin
adapter differences, not divergent workflow logic.

The installation model for non-development usage is:

1. install via Homebrew or `cargo install`
2. run `pg-logstats agent install`
3. let the agent use the installed guidance

The intended installation path does not assume a repository checkout.

### 9. Documentation Stance

`docs/` is for humans setting up `pg-logstats` for agents.

That means the docs should focus on:

- why to install `pg-logstats` for agent-driven PostgreSQL triage
- how to install it
- how to install the agent guidance
- how to verify it is being used correctly
- what workflows the agent is automating
- where runbook ends and judgement begins
- when the product will stop and require escalation

The docs should not optimize for human command-by-command CLI usage.

In particular:

- detailed command reference for workflows such as `top query-families` is not a
  primary docs goal
- workflow and algorithm reference is desirable so expert humans can audit what
  the agent is automating
- root docs should be hyper-focused on agent setup, trust, and quick start

The `agents/` directory in this repository serves development of `pg-logstats`
itself. It is not the end-user installation model.

### 10. Output Contract Implications

Because the agent is the primary user, outputs must optimize for:

- compactness
- determinism
- explicit readiness
- explicit action validity
- auditable evidence
- explicit stop or escalate outcomes

The report must tell the agent:

- whether the workflow is allowed
- why it is allowed or blocked
- what branch can be taken next
- what cannot be done
- when the workflow must stop

Free-form prose is insufficient for these decisions. The structured contract is
part of the product, not an implementation detail.

## Consequences

### Positive

- The project now has a sharper reason to exist next to `pgBadger`.
- Product decisions can optimize for agent safety and control rather than
  generic CLI convenience.
- The docs and site can focus on setup, trust, and auditability instead of
  command manuals.
- Workflow design can be validated against recognized PostgreSQL runbooks.
- The installation story becomes clearer: install the tool, then install the
  agent guidance.

### Negative

- The project becomes intentionally less attractive as a manual human CLI.
- Some previously plausible degraded or live-only flows are no longer acceptable
  as beta success paths.
- The gateway policy and SQL action catalog now carry more product weight and
  therefore need stronger tests and clearer docs.

## Implementation Consequences

This decision puts immediate pressure on the following areas:

1. `inspect` must gate workflows honestly and exit when prerequisites are not
   met
2. `run-sql` must remain action-ID-driven rather than arbitrary-SQL-driven
3. agent install must be treated as a first-class shipped feature
4. README and docs index should be rewritten around agent setup and value
5. workflow docs should explain the automated runbook and judgement boundary,
   not teach manual CLI usage
6. tests should protect the JSON action contract and the bounded investigation
   loop
7. dry-run investigations should be checked against credible pgBadger-style
   operator practice, starting with slow-query triage

## Follow-Up Decisions Still Needed

This ADR intentionally leaves some details open:

- the exact minimum readiness checklist for each workflow
- the exact built-in SQL action catalog for each finding type
- the precise install layout for each supported agent harness
- the exact test harness for validating agent guidance end to end
- the release and packaging details after the Homebrew path is added
