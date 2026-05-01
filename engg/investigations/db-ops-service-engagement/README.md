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
  - [Machine evidence assembly](deep-dives/machine-evidence-assembly.md)
  - [Deterministic analysis of machine evidence](deep-dives/deterministic-analysis-of-machine-evidence.md)
  - [Context evidence capture](deep-dives/context-evidence-capture.md)
  - [Company-aware diagnosis](deep-dives/company-aware-diagnosis.md)
  - [Recommendation and decision framing](deep-dives/recommendation-and-decision-framing.md)
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
- The operating model is layered. Scripts and OSS tooling create trusted
  machine evidence; agents reduce walkthrough and context-gathering friction;
  humans own severity, safety, validation, and production-impacting decisions.
- Agent work should be scored against the X-reference test: bounded,
  checkable, structured, and verifiable. Deep dives classify each agent step as
  `No risk`, `Risk`, or `Unknown`, then explain why.

| Step | Evidence or insight | Primary accelerator | Human boundary | Output |
| --- | --- | --- | --- | --- |
| Intake and scope gate | Case readiness and likely incident class | OSS findings plus agent-generated intake brief | Accept, reject, or re-scope | Intake state and first branch |
| Machine evidence assembly | Logs, metrics, stats, pooler, replica, CDC, baseline windows | OSS collectors and scripts; agent-guided collection walkthrough | Data sharing, redaction, live SQL safety | Evidence bundle and completeness report |
| Deterministic analysis | Ranked machine findings | Scripts and utilities such as `pg-logstats`; agent-driven tool loop | Artifact review and live SQL approval | Ranked findings and missing evidence |
| Context evidence capture | Owners, deploys, runbooks, business impact, operator heuristics | Agent-assisted gathering from docs, links, exports, walkthroughs, and targeted questions | Context validation by operators | Context pack and ownership map |
| Company-aware diagnosis | Joined machine and context evidence; prioritized causal chain | Agent-assisted synthesis and weighting loop | Validate priority, causal chain, and confidence | Diagnosis memo |
| Recommendation framing | Action choice, risk, reversibility, deferral, permanent change | Agent-assisted tradeoff framing plus senior judgment | Approve action path and production-impacting changes | Recommendation brief and follow-up plan |

| Step | Agent risk summary |
| --- | --- |
| Intake and scope gate | Mostly low risk, except first-branch selection can bias the engagement and needs human approval. |
| Machine evidence assembly | Collection guidance and fallback baselines are risky until recipes and baseline rules are tested; gap explanation is low risk. |
| Deterministic analysis | Agent-driven tool loops are useful but risky when selecting windows, thresholds, or filters; summaries are low risk when source-linked. |
| Context evidence capture | Gathering and drafting are useful, but claim extraction and ownership mapping need validation; local mapping reliability is still unknown. |
| Company-aware diagnosis | High-risk agent step because weighting machine severity against business context can change priority; humans must validate. |
| Recommendation framing | High-risk where actions are classified or compared; lower risk for formatting briefs and listing assumptions. |

- OSS tooling speeds up the machine-evidence side because the customer can run
  it locally before the engagement. This reduces access friction, preserves
  privacy, makes the case concrete, and prevents expert time from being spent on
  parseability, completeness, or basic log triage.
- Agents are useful where the workflow is structured but customer-specific.
  They can guide provider-specific evidence collection, turn messy notes into
  an intake brief, drive deterministic tools like `pg-logstats`, extract claims
  from docs, request the right links or exports, detect contradictions, and ask
  targeted follow-up questions during operator walkthroughs.
- Humans remain the control point for judgment. They decide whether the case is
  valuable, whether the evidence is good enough, whether context is accurate,
  whether a recommendation is safe, and whether production-impacting action is
  approved.
- Diagnosis should not blindly follow the highest machine-severity finding. The
  priority comes from a loop that weighs deterministic machine findings against
  ownership, business impact, deploy timing, known noise, and operator
  validation.
- Recommendations should separate immediate mitigation, root-cause repair,
  diagnostic follow-up, actions to avoid, and structural changes. The agent can
  frame the tradeoffs quickly, but humans own the final action path.
- The current scoring pattern is intentional:
  - `No risk`: summary, routing, formatting, and question-generation tasks that
    are source-linked and reviewable.
  - `Risk`: branch selection, baseline choice, threshold tuning, suppression,
    weighting, and action tradeoffs that can bias the engagement.
  - `Unknown`: local mappings or reusable context packs where design-partner
    evidence is needed before deciding reliability.
- The time target assumes the evidence package is ready:
  - T+0 to T+30m: intake and scope gate
  - T+30m to T+90m: machine evidence assembly
  - T+90m to T+150m: deterministic analysis
  - T+150m to T+210m: context evidence capture
  - T+210m to T+270m: diagnosis
  - T+270m to T+330m: recommendation draft
  - T+330m to T+360m: customer review and revision
- One day remains the outer bound when evidence is incomplete, causal chains are
  ambiguous, or ownership context is hard to reconstruct.
- Re-scope the engagement when the customer cannot provide logs or metrics for
  the relevant window, no operator is available for context, the request is
  really a generic health check, or the customer expects autonomous production
  changes.

## Differentiation

- OSS scripts speed up intake and engagement.
  - Customers can run tools locally before granting broad access.
  - Machine evidence becomes concrete before expert time starts.
  - Parseability, completeness, redaction, and basic ranking become
    deterministic and reviewable.
  - `pg-logstats` is the first concrete trial of this pattern.
- Agents speed up the complete engagement when work is bounded, checkable,
  structured, and verifiable.
  - Agents guide collection walkthroughs and reduce customer/operator friction.
  - Agents drive deterministic tool loops without becoming the source of truth.
  - Agents gather docs, links, exports, and commentary into reviewable context
    packs.
  - Agents draft diagnosis and recommendation artifacts, while humans validate
    weighting, safety, and action choices.

## Design Partner Workflow

- The design partner goal is to validate three claims:
  - production Postgres db-ops requires professional services for hard
    incidents
  - OSS scripts speed up intake and machine evidence assembly
  - agents speed up the complete engagement when their work is bounded,
    checkable, structured, and verifiable
- A strong design partner has meaningful production scale, recurring database
  operations pain, access to historical incident artifacts, and named operators
  who can walk through real investigations.
- The trial should start with historical incidents, then move to live or
  near-live cases only after the workflow produces useful, reviewable artifacts.
- The design partner review should measure:
  - whether services would have materially helped
  - whether OSS findings made intake faster and more concrete
  - whether agents saved time or added correction burden at each stage
  - which agent steps should be `No risk`, `Risk`, or `Unknown`
- The core learning goals are:
  - which evidence is always available
  - which evidence is missing or unreliable
  - which context changes diagnosis
  - where deterministic tooling is enough
  - where model or human judgment is needed
  - which artifact format is useful during incident response
