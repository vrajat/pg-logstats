# Database Operations Service Engagement

- Status: living investigation
- Date: 2026-05-01

## Purpose

- Define a professional service engagement for production database operations.
  The investigation should test whether high-value recommendations can be
  delivered in a few hours, with one day as the outer bound.
- Treat `pg-logstats` as the first concrete trial, not the whole category. It is
  useful because it can produce deterministic evidence, but the service thesis
  is broader than log analysis or Postgres alone.
- Focus first on Postgres-backed production systems where database behavior
  threatens SLA, cost, or uptime.
- Optimize for incidents and chronic operational pain that internal teams have
  not resolved. The bar is senior diagnosis and decision framing, not 101-level
  database advice.

## Navigation

- Level 2 docs:
  - [Successful engagement criteria, workflow, and timeline](successful-engagement.md)
  - [Design partner workflow](design-partner-workflow.md)
- Level 3 deep dives:
  - [Intake and triage](deep-dives/intake-and-triage.md)
  - [Machine evidence and analysis](deep-dives/machine-evidence-and-analysis.md)
  - [Context evidence capture](deep-dives/context-evidence-capture.md)
  - [Company-aware recommendation](deep-dives/company-aware-recommendation.md)
  - [Evidence required before diagnosis](deep-dives/evidence-required-before-diagnosis.md)
- Supporting notes:
  - [Research notes](research-notes.md)

## Successful Engagement

- The target customer is an IT, platform, SRE, or database team that has already
  tried to solve the issue internally. They need faster diagnosis, better
  prioritization, and clearer action framing.
- The target environment is a production installation with operational
  complexity:
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
| Intake and scope gate | Case readiness and likely incident class | OSS findings plus structured intake brief | Accept, reject, or re-scope | Intake state and first branch |
| Machine evidence and analysis | Logs, metrics, stats, pooler, replica, CDC, baseline windows, ranked findings | OSS collectors, scripts, and utilities such as `pg-logstats` | Data sharing, redaction, artifact review, live SQL safety | Evidence bundle, ranked findings, and missing evidence |
| Context evidence capture | Owners, deploys, runbooks, business impact, operator heuristics | Source-linked gathering from docs, links, exports, walkthroughs, and targeted questions | Context validation by operators | Context pack and ownership map |
| Company-aware recommendation | Joined machine and context evidence; action choice, risk, reversibility, deferral, permanent change | Tradeoff framing plus senior judgment | Validate priority, confidence, and production-impacting changes | Recommendation brief and follow-up plan |

| Step | Agent risk summary |
| --- | --- |
| Intake and scope gate | Mostly low risk, except first-branch selection can bias the engagement and needs human approval. |
| Machine evidence and analysis | Collection guidance is risky until recipes are tested; gap explanation and source-linked walkthroughs are low risk. |
| Context evidence capture | Gathering and drafting are useful, but claim extraction and ownership mapping need validation; local mapping reliability is still unknown. |
| Company-aware recommendation | High-risk where priority is weighted or actions are compared; lower risk for assumptions, falsification checks, and brief drafting. |

- OSS tooling may speed up the machine-evidence side because the customer can
  run it locally before the engagement. This should reduce access friction,
  preserve privacy, and make the case concrete.
- Agents are candidates where the workflow is structured but customer-specific.
  They can guide provider-specific evidence collection, turn messy notes into
  an intake brief, summarize deterministic outputs, extract claims
  from docs, request the right links or exports, detect contradictions, and ask
  targeted follow-up questions during operator walkthroughs.
- Humans remain the control point for judgment. They decide whether the case is
  valuable, whether the evidence is good enough, whether context is accurate,
  whether a recommendation is safe, and whether production-impacting action is
  approved.
- The diagnosis inside the recommendation should not blindly follow the highest
  machine-severity finding. Priority comes from weighing deterministic machine
  findings against ownership, business impact, deploy timing, known noise, and
  operator validation.
- Recommendations should separate immediate mitigation, root-cause repair,
  diagnostic follow-up, actions to avoid, and structural changes. Humans own
  the final action path.
- The current scoring pattern is intentional:
  - `No risk`: summary, routing, formatting, and question-generation tasks that
    are source-linked and reviewable.
  - `Risk`: branch selection, baseline choice, threshold tuning, suppression,
    weighting, and action tradeoffs that can bias the engagement.
  - `Unknown`: local mappings or reusable context packs where design-partner
    evidence is needed before deciding reliability.
- The time target assumes the evidence package is ready:
  - T+0 to T+30m: intake and scope gate
  - T+30m to T+150m: machine evidence and analysis
  - T+150m to T+210m: context evidence capture
  - T+210m to T+330m: company-aware recommendation draft
  - T+330m to T+360m: customer review and revision
- One day remains the outer bound when evidence is incomplete, causal chains are
  ambiguous, or ownership context is hard to reconstruct.
- Re-scope the engagement when the customer cannot provide logs or metrics for
  the relevant window, no operator is available for context, the request is
  really a generic health check, or the customer expects autonomous production
  changes.

## Differentiation

- OSS scripts may speed up intake and engagement.
  - Customers can run tools locally before granting broad access.
  - Machine evidence becomes concrete before expert time starts.
  - Parseability, completeness, redaction, and basic ranking become
    deterministic and reviewable.
  - `pg-logstats` is the first concrete trial of this pattern.
- Agent acceleration is unproven; test it only where work is bounded,
  checkable, structured, and verifiable.
  - Agents guide collection walkthroughs and reduce customer/operator friction.
  - Agents prepare walkthroughs of deterministic outputs without becoming the
    source of truth.
  - Agents gather docs, links, exports, and commentary into reviewable context
    packs.
  - Agents draft recommendation artifacts, while humans validate weighting,
    safety, and action choices.

## Design Partner Workflow

- The design partner goal is to validate three claims:
  - production Postgres db-ops requires professional services for hard
    incidents
  - OSS scripts speed up intake and machine evidence
  - which agent steps, if any, save time without adding correction burden
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
