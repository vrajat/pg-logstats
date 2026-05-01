# Machine Evidence Assembly

Status: living deep dive  
Parent: [Successful engagement criteria, workflow, and timeline](../successful-engagement.md)

## Question

How should a high-speed database operations engagement collect machine evidence,
and where, if anywhere, is agent assistance useful?

## Current Framing

Machine evidence assembly should be mostly deterministic.

The goal is not to interpret the incident. The goal is to produce a complete,
normalized, redacted, and reviewable evidence bundle that can feed deterministic
analysis and later be joined with context evidence.

## What Should Be Deterministic

The core work should be scripting, utilities, and structured validation:

- collect logs
- collect metrics exports
- collect `pg_stat_*` snapshots
- collect `pg_stat_statements` when available
- collect pooler stats
- collect replica state
- collect CDC or logical replication state
- normalize timestamps
- normalize incident and baseline windows
- check parseability
- check completeness
- redact sensitive fields
- preserve stable grouping keys after redaction
- produce a manifest
- report missing or low-quality evidence
- generate stable findings from tools like `pg-logstats`

This should be deterministic because the output needs to be trusted,
repeatable, and easy to challenge.

## Where Exceptions And Customization Enter

There are always environment-specific choices:

- cloud provider: RDS, Aurora, Cloud SQL, self-hosted, Kubernetes
- log access path: local files, object storage, cloud logging, database service
  export, or customer observability platform
- metric source: Prometheus, Datadog, CloudWatch, Stackdriver, vendor console,
  screenshots, or CSV exports
- available statistics: `pg_stat_statements`, queryid, logs only, sampled logs,
  or custom tracing
- topology: primary, replicas, poolers, CDC consumers, sharded systems, or
  analytics followers
- incident window: customer-provided, alert-derived, inferred from symptoms, or
  adjusted for timezone mismatches
- redaction policy: remove literals, hash identifiers, preserve table names, or
  hash table names

These choices do not require diagnosis, but they do require workflow guidance
and sometimes operator confirmation.

## Agent Workflow Opportunity

The agent should act as a guided collection assistant, not a diagnostic judge.

Useful work:

- select the right collection recipe based on provider and topology
- turn the intake state into a concrete collection checklist
- generate read-only SQL snippets when needed
- explain why a missing artifact matters
- detect incomplete or inconsistent uploads
- identify when redaction destroyed useful grouping keys
- suggest fallback baseline windows when none were provided
- route missing application or ownership facts to context evidence capture
- produce a final evidence manifest and completeness report

The agent should not decide the root cause during this step.

## Human Approval Boundaries

Human approval is still needed for safety and data handling:

- approving data sharing
- approving redaction policy
- approving any live read-only SQL during an ongoing incident
- approving access to restricted logs or observability systems
- deciding whether fallback baselines are acceptable
- deciding whether the evidence is complete enough to start deterministic
  analysis

This is operator confirmation, not open-ended expert diagnosis.

## Evidence Bundle Output

The bundle should be easy to inspect and transfer.

Expected contents:

- manifest
- collection timestamp
- incident window
- baseline windows
- source inventory
- collector versions
- redaction policy
- log-derived findings
- statistics snapshots
- pooler snapshots
- replica and CDC snapshots
- metric exports or references
- parseability report
- completeness report
- known gaps

## Completion Criteria

Machine evidence assembly is complete when:

- the incident window is covered
- at least one baseline is available or explicitly marked missing
- timestamps are normalized
- redaction policy is recorded
- major data sources are either present or explicitly missing
- the bundle can be analyzed offline where possible
- missing context has been routed to context evidence capture
- a human has approved any data-sharing or live-query boundary

## Agent Step Risk Analysis

Scores:

- `No risk`: bounded, checkable, structured, and verifiable enough for agent
  acceleration with normal human review.
- `Risk`: useful, but can bias the engagement if not reviewed or constrained.
- `Unknown`: not enough examples yet to decide whether the agent role is
  reliable.

| Agent step | Score | Why |
| --- | --- | --- |
| Guide the operator through the provider and topology-specific collection recipe | Risk | Provider details vary and bad guidance can waste time or collect the wrong data; tested recipes and human approval reduce the risk. |
| Suggest fallback baseline windows when none were provided | Risk | Baseline choice can bias analysis, so the suggestion must be recorded and approved. |
| Explain missing or low-trust signals | No risk | This is bounded by deterministic completeness and parseability reports. |
| Route context gaps to context evidence capture | No risk | This only creates follow-up questions and does not decide the diagnosis. |

Information that would improve the scores:

- provider-specific collector recipes
- a minimum evidence bundle schema
- parseability and completeness tests
- redaction validation examples
- sample bundles from real incidents

## Working Thesis

Machine evidence assembly does not need diagnostic judgment. It needs
deterministic collection, strong validation, and a good guided walkthrough for
environment-specific paths.

The agent opportunity is orchestration: make collection faster, reduce mistakes,
explain gaps, and hand deterministic analysis a clean evidence bundle.
