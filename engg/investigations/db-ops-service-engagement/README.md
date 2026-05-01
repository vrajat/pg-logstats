# Database Operations Service Engagement

- Status: living investigation
- Date: 2026-05-01

## Purpose

- Define a professional service engagement for production database operations.
  The engagement should deliver high-value recommendations in a few hours, with
  one day as the outer bound.
- Treat `pg-logstats` as the first concrete trial, not the whole category. It is
  useful because it can produce deterministic evidence, but the service thesis
  is broader than log analysis or Postgres alone.
- Focus on production systems where database behavior threatens SLA, cost, or
  uptime. The target includes Postgres-backed applications, data integration
  systems, analytics stacks, and high-throughput key-value or event stores.
- Optimize for incidents and chronic operational pain that internal teams have
  not resolved. The bar is senior diagnosis and decision framing, not 101-level
  database advice.

## Navigation

- Level 2 docs:
  - [Successful engagement criteria, workflow, and timeline](successful-engagement.md)
  - [Design partner workflow](design-partner-workflow.md)
- Level 3 deep dives:
  - [Intake and triage](deep-dives/intake-and-triage.md)
  - [Evidence required before diagnosis](deep-dives/evidence-required-before-diagnosis.md)
- Supporting notes:
  - [Research notes](research-notes.md)

## Successful Engagement

- The target customer is an IT, platform, SRE, or database team that has already
  tried to solve the issue internally. They need faster diagnosis, better
  prioritization, and clearer action framing.
- The target environment is a serious production installation:
  - terabytes of data
  - high query volume
  - read replicas
  - CDC or logical replication
  - connection pooling
  - backups and restore validation
  - failover mechanisms
  - separate app, infra, data, and database owners
- The engagement should produce a recommendation brief, not a broad report. It
  should identify what is likely happening, what evidence supports it, what to
  do now, what not to do yet, what evidence could change the recommendation,
  and what permanent changes should follow.
- The workflow has five stages:
  - intake and scope gate
  - evidence assembly
  - deterministic analysis
  - company-aware diagnosis
  - recommendation and decision framing
- Intake should use OSS-generated machine evidence as a case-shaping input, not
  as the final decision. The intake decision still needs human judgment on
  severity, service fit, operator availability, production safety, and whether
  recommendations would be actionable.
- The workflow should keep rule-based evidence collection separate from LLM or
  human judgment. This avoids "AI everywhere" architecture and makes the service
  easier to trust.

| Stage | Mostly deterministic | LLM or judgment useful for | Human approval boundary |
| --- | --- | --- | --- |
| Intake | parse OSS findings; classify incident type; check evidence readiness | detect contradictions; draft targeted questions; suggest first branch | accept, reject, or re-scope engagement |
| Evidence assembly | collect logs, stats, metrics; score bundle completeness | identify missing context; generate targeted follow-ups | approve data sharing and redaction |
| Deterministic analysis | rank query families, waits, errors, lag, pool saturation | summarize patterns; compare plausible explanations | none if read-only or offline |
| Company-aware diagnosis | join known service names, deploy timestamps, runbooks | reason over ownership, business impact, prior incidents | validate context and causal chain |
| Recommendation framing | list candidate mitigations and known risk classes | prioritize actions; frame reversibility; decide what not to do | approve any production-impacting action |

- Deterministic analysis should compress machine evidence into ranked findings.
  Inputs include logs, metrics, system views, pooler data, replication state,
  CDC state, and baseline comparisons.
- Company-aware diagnosis should join those findings with context that machines
  do not naturally know: application ownership, deploy history, runbooks,
  product criticality, known noisy jobs, previous incidents, and operator
  heuristics.
- The time target assumes the evidence package is ready:
  - T+0 to T+30m: intake and scope gate
  - T+30m to T+90m: evidence assembly
  - T+90m to T+150m: deterministic analysis
  - T+150m to T+210m: diagnosis
  - T+210m to T+270m: recommendation draft
  - T+270m to T+360m: customer review and revision
- One day remains the outer bound when evidence is incomplete, causal chains are
  ambiguous, or ownership context is hard to reconstruct.
- Re-scope the engagement when the customer cannot provide logs or metrics for
  the relevant window, no operator is available for context, the request is
  really a generic health check, or the customer expects autonomous production
  changes.

## Design Partner Workflow

- The design partner goal is to discover the real operational workflow. We are
  not trying to validate a polished demo; we are trying to learn how expert
  teams actually investigate hard database incidents.
- A strong design partner has meaningful production scale, recurring database
  operations pain, access to historical incident artifacts, and named operators
  who can walk through real investigations.
- The first trial should use one or two historical incidents where the internal
  team struggled. Historical incidents let us compare the service output against
  ground truth without adding live-incident risk.
- Request the artifacts needed to reconstruct the investigation:
  - incident timeline
  - production topology
  - logs and metrics
  - SQL snapshots
  - deploy and migration history
  - runbooks
  - incident notes
  - unresolved questions
- Require a walkthrough with the people who did the work. The decisive context
  often lives in undocumented Slack habits, dashboard sequences, shell commands,
  ownership conventions, and judgment calls.
- Run an evidence-package trial before the timed investigation. The partner
  should assemble a sanitized incident package, and we should measure how long
  that takes. Evidence assembly time is part of the service design problem.
- Run the timed investigation with clear safety boundaries:
  - no autonomous production changes
  - human approval for live SQL
  - every claim tied to evidence
  - immediate actions separated from short-term and structural actions
  - unknowns called out explicitly
- Review the output against ground truth. The review should answer whether the
  workflow found the issue faster, surfaced missed context, avoided unsafe
  actions, and produced recommendations that senior operators would use.
- The core learning goals are:
  - which evidence is always available
  - which evidence is missing or unreliable
  - which context changes diagnosis
  - where deterministic tooling is enough
  - where model or human judgment is needed
  - which artifact format is useful during incident response
