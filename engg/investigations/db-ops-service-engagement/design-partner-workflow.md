# Design Partner Workflow

Status: living investigation  
Parent: [Database Operations Service Engagement](README.md)

## Design Partner Goal

- Validate whether production Postgres database operations is a professional
  services problem.
- Validate whether OSS scripts and tools can speed up intake and machine
  evidence assembly.
- Validate whether agents can speed up steps across the engagement without
  creating unbounded, unverifiable, or unsafe work.
- Use `pg-logstats` as the first concrete OSS trial, not as the full service
  boundary.

## Core Questions

| Question | What We Need To Learn | Evidence |
| --- | --- | --- |
| Does Postgres db-ops require professional services? | Whether hard incidents depend on missing context, expert judgment, cross-team coordination, and action framing that tooling alone cannot provide. | Historical incidents where internal teams struggled, delayed, or produced unclear recommendations. |
| Do OSS scripts speed up intake and engagement? | Whether customer-run tools can produce trusted machine evidence before experts join. | Time to install/run, evidence completeness, parseability, quality of findings, and privacy/access friction. |
| Do agents speed up the complete engagement? | Whether agents reduce time across intake, collection, analysis, context capture, diagnosis, and recommendations while staying bounded and reviewable. | Per-step timing, artifact quality, human corrections, risk scores, and operator trust. |

## Design Partner Profile

- Strong candidates:
  - production Postgres at meaningful scale
  - recurring SLA, cost, uptime, replica, CDC, or query-performance incidents
  - internal team has already tried to diagnose hard cases
  - access to historical incident artifacts
  - willingness to run OSS tools locally
  - named operators available for walkthroughs and validation
  - ability to share sanitized logs, metrics, timelines, docs, and runbooks
- Weak candidates:
  - toy workloads
  - generic observability interest without incident pain
  - no historical incidents
  - no access to raw or sanitized evidence
  - no operator availability
  - desire for a generic dashboard or report

## Trial Shape

- Start with one or two historical incidents.
  - The internal team should have struggled or taken longer than desired.
  - Ground truth should exist or be reconstructable.
  - Historical incidents reduce live-incident risk while preserving realism.
- Repeat later on a live or near-live degradation.
  - Use this only after the historical workflow has produced useful artifacts.
  - Test speed under pressure, trust, and data-access friction.

## Phase 1: Professional Services Need

- Goal:
  - determine whether the incident class requires services, not just tooling
- Request:
  - incident summary
  - original timeline
  - internal RCA or unresolved questions
  - what the team tried
  - where the team got stuck
  - who needed to be involved
  - what decision was hard to make
- Evaluate:
  - was the hard part machine evidence, context, prioritization, or action
    framing?
  - did the team need cross-functional ownership or business tradeoffs?
  - did generic tooling already exist but fail to produce a decision?
  - would a recommendation brief have been valuable during the incident?
- Success signal:
  - the partner can name incidents where faster expert diagnosis and decision
    framing would have materially helped.

## Phase 2: OSS Script Trial

- Goal:
  - determine whether customer-run OSS tools can speed up intake and machine
    evidence assembly
- Ask the partner to run the OSS path locally:
  - install or build `pg-logstats`
  - run it against sanitized incident and baseline windows
  - generate findings
  - generate parseability and completeness notes where available
  - record setup time, run time, failures, and missing data
- Measure:
  - time from instructions to first usable artifact
  - whether logs were parseable
  - whether baseline windows were easy to provide
  - whether redaction preserved useful grouping
  - whether findings were specific enough for intake
  - whether the customer avoided sharing raw logs
- Success signal:
  - OSS output makes intake more concrete and reduces expert time spent on
    basic evidence readiness.
- Failure signal:
  - setup, log format, redaction, or missing data makes the OSS path slower than
    a guided services workflow.

## Phase 3: Agent Acceleration Trial

- Goal:
  - determine whether agents speed up the full engagement while staying
    bounded, checkable, structured, and verifiable
- Run the same incident through the staged workflow:
  - intake and triage
  - machine evidence assembly
  - deterministic analysis
  - context evidence capture
  - company-aware diagnosis
  - recommendation framing
- For each stage, record:
  - agent task
  - `No risk`, `Risk`, or `Unknown` score
  - human corrections
  - time saved or time added
  - artifact quality
  - missing structure or missing evidence
- Success signal:
  - agents reduce operator and expert time without hiding assumptions or
    producing unverifiable claims.
- Failure signal:
  - agents produce broad narrative, unsupported claims, unsafe action framing,
    or require more correction than the time they save.

## Phase 4: Ground Truth Review

- Goal:
  - compare the service workflow against what actually happened
- Ask:
  - did OSS findings surface the important machine signals?
  - did the agent-generated intake brief focus the discussion?
  - did the context pack capture the facts operators considered decisive?
  - did the diagnosis prioritize the right issue?
  - did the recommendation separate mitigation, repair, deferral, and actions
    to avoid?
  - which agent steps were reliable?
  - which agent steps required too much correction?
  - where did human judgment remain essential?
- Required outputs:
  - corrected artifacts
  - timing comparison
  - agent risk-score revisions
  - list of scripts or docs needed before the next trial

## Phase 5: Live Or Near-Live Trial

- Goal:
  - test the workflow under realistic time pressure
- Preconditions:
  - historical trial produced a useful recommendation artifact
  - OSS evidence path is documented enough to run quickly
  - context pack shape is understood
  - human approval boundaries are explicit
- Measure:
  - time to intake state
  - time to evidence bundle
  - time to ranked findings
  - time to context pack
  - time to diagnosis
  - time to recommendation brief
  - operator trust in the artifact

## Metrics To Track

- Professional services need:
  - incident duration before useful diagnosis
  - number of teams involved
  - unresolved RCA questions
  - decisions that were delayed or contentious
- OSS script value:
  - setup time
  - run time
  - parseability success
  - completeness score
  - redaction success
  - usefulness of findings for intake
- Agent acceleration:
  - time saved per workflow stage
  - number of human corrections
  - risk score per agent step
  - unsupported claims caught
  - operator trust score
  - artifact reuse in incident channel or follow-up tickets

## Initial Hypotheses

- Postgres db-ops at scale is often a services problem because the hard part is
  joining machine evidence with company context and action tradeoffs.
- OSS scripts can materially speed up intake and machine evidence assembly if
  they are easy to run locally and produce stable, shareable findings.
- Agents can speed up the engagement when the work is bounded, checkable,
  structured, and verifiable.
- Agents are most reliable for summarization, routing, collection walkthroughs,
  source-linked extraction, and artifact drafting.
- Agents are riskiest when they choose branches, tune thresholds, suppress
  findings, weight priorities, or frame production-impacting actions.
- The design partner trial should produce evidence about which agent steps are
  reliable, risky, or still unknown.
