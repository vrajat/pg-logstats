# Successful Engagement Criteria, Workflow, And Timeline

Status: living investigation  
Parent: [Database Operations Service Engagement](README.md)

## Goal

Define the criteria, workflow, and timeline for testing whether a professional
service engagement can deliver high-value production database operations
recommendations in a few hours, with one day as the outer bound.

## Operating Definition

### Database Operations

For this effort, database operations means investigation of production incidents
or chronic production pain involving:

- SLA or SLO degradation
- uptime risk
- cost escalation
- primary or replica saturation
- replication or CDC lag
- query-plan regressions
- connection pool exhaustion
- lock contention
- write amplification
- storage growth, bloat, vacuum, and WAL pressure
- operational risk around failover, migrations, and maintenance

### Production Installation

The target environment is not a small application database. It likely includes
some of:

- read replicas
- CDC or logical replication
- connection pooling
- backups and restore validation
- separate operational owners for app, infra, data, and database layers
- incomplete documentation and some disagreement between observed reality and
  internal runbooks

### Professional Service Engagement

The customer is an IT, platform, SRE, or database team that has already tried to
solve the issue internally. The engagement must avoid basic advice and instead
produce senior-level diagnosis, mitigation framing, and investment guidance.

### Successful Outcome

Success means high-value recommendations faster than the customer's current
path.

The output should answer:

- what is most likely happening?
- what evidence supports that view?
- what should be done immediately?
- what should not be done yet?
- what additional data would change the recommendation?
- what permanent changes should follow if the diagnosis holds?

The engagement is not successful if it only produces a broad report, generic
best practices, or a list of possible causes without prioritization.

## Success Criteria

An engagement is successful when it delivers all of the following:

- a recommendation brief with diagnosis tied to specific evidence
- a short list of immediate mitigations with reversibility and risk called out
- a clear distinction between incident mitigation, root-cause repair, and
  longer-term platform investment
- owner-aware recommendations across database, application, infrastructure, and
  data teams
- explicit confidence levels and missing evidence
- an artifact the customer can put in an incident channel or executive update
- follow-up work that is specific enough to become tickets or runbook changes

The recommendation quality bar is principal-engineer level, not generic DBA
triage.

## Non-Goals

The engagement should not optimize for:

- replacing the customer's incident commander
- autonomously changing production
- producing a comprehensive health audit
- teaching Postgres basics
- building dashboards
- recreating pgBadger as a prettier report
- making `pg-logstats` the center of the story

## Evidence Required Before Diagnosis

Deep dive: [Evidence required before diagnosis](deep-dives/evidence-required-before-diagnosis.md)

Diagnosis requires two kinds of evidence:

- machine evidence: logs, metrics, SQL snapshots, statistics views, pooler
  state, replication state, CDC state, and outputs from tools like
  `pg-logstats`
- context evidence: application ownership, topology, deploy history, runbooks,
  incident history, business impact, known noisy jobs, and operator heuristics

Machine evidence describes what changed. Context evidence explains what that
change means, who owns it, and which mitigations are safe.

The minimum machine evidence package should include:

- incident timeline with alert times and mitigation attempts
- PostgreSQL version, hosting model, topology, and failover mechanism
- primary and replica roles, read-routing rules, and CDC topology
- relevant Postgres logs for the incident window and nearby baseline windows
- `pg_stat_statements` snapshot or export when available
- active session and lock snapshots if captured
- replica lag, WAL, checkpoint, vacuum, and IO metrics
- connection pool metrics

The minimum context evidence package should include:

- customer impact and business criticality
- deploy, migration, feature-flag, and bulk-job timeline
- application ownership map for major query families or service names
- current runbooks and known-stale docs
- recent similar incidents and unresolved RCA notes
- known-benign maintenance, ETL, export, and backfill workloads
- operator walkthrough of dashboards, commands, and judgment calls

The evidence package must distinguish observed facts from operator guesses.

## Engagement Workflow

Step tags:

- `[script]`: deterministic collection, validation, scoring, or transformation
- `[agent]`: guided workflow, summarization, contradiction detection, or
  targeted follow-up generation
- `[human]`: severity judgment, context validation, safety approval, or
  engagement decision

Agent-fit scoring:

- Score every reviewed agent step against whether it is bounded, checkable,
  structured, and verifiable.
- Use `No risk` when the task is source-linked, reviewable, and cannot directly
  bias diagnosis or action.
- Use `Risk` when the task can steer the investigation, hide or amplify a
  signal, change priority, or affect the action path.
- Use `Unknown` when design-partner examples are needed before the agent role
  can be trusted.
- When a step is `Risk` or `Unknown`, state the human review, artifact shape, or
  examples needed to make the step safe.

Workflow risk rollup:

| Step | `No risk` agent work | `Risk` agent work | `Unknown` agent work |
| --- | --- | --- | --- |
| Intake and triage | Summaries, contradiction lists | First investigation branch | None identified yet |
| Machine evidence and analysis | Missing-signal explanation, findings walkthrough, routing follow-ups | Provider-specific guidance | None identified yet |
| Context evidence capture | Ingestion, commentary structuring, context-pack drafting | Fact extraction | Ownership and product-path mapping reliability |
| Company-aware recommendation | Assumptions, unknowns, falsification checks, draft artifacts | Timeline correlation, noise suppression, priority weighting, action classification | Service/job/product-path mapping reliability |

### 0. Intake And Triage

Deep dive: [Intake and triage](deep-dives/intake-and-triage.md)

Goal: decide whether the case is eligible for a high-speed engagement.
This step prevents wasting the engagement on cases that lack severity, evidence,
or access to the people who can validate context.

Work:

- `[script]` review OSS-generated findings or machine-evidence summaries when
  available
- `[script]` confirm machine-evidence readiness: logs, stats, metrics, baselines, and
  parseability
- `[agent]` summarize the case, contradictions, gaps, and first investigation
  branch
- `[human]` confirm severity, service fit, context readiness, and safety
  constraints
- `[human]` decide whether the case is ready, needs machine evidence, needs context
  evidence, or should be rejected/re-scoped

Exit artifact:

- one-page intake brief
- machine-evidence readiness score
- context-evidence readiness score
- initial hypothesis map
- intake state: `Ready`, `Needs machine evidence`, `Needs context evidence`, or
  `Reject / re-scope`

### 1. Machine Evidence And Analysis

Deep dive: [Machine evidence and analysis](deep-dives/machine-evidence-and-analysis.md)

Goal: make the machine side of the problem bounded, checkable, structured, and
reviewable. This step collects the evidence, validates it, and compresses it
into ranked findings that can be inspected and joined with context.

Work:

- `[agent]` guide the operator through the correct collection recipe for the
  provider, topology, and available data sources
- `[script]` assemble an evidence bundle with logs, metrics, normalized
  windows, baselines, and topology notes
- `[human]` approve data sharing, live read-only SQL, and redaction boundaries
- `[script]` validate, redact, and reduce the bundle into ranked,
  source-linked findings
- `[agent]` prepare a findings walkthrough and separate findings from hypotheses
- `[human]` review the artifact for credibility, collection artifacts, and live
  SQL safety
- `[agent]` route missing evidence and context questions to the next workflow
  step

`pg-logstats` can be one tool in this phase, especially for compact log-derived
findings. It should not be the only evidence path.

Exit artifact:

- evidence manifest and quality notes
- ranked findings with source references
- missing-evidence list, suggested follow-up SQL, and context questions

### 2. Context Evidence Capture

Deep dive: [Context evidence capture](deep-dives/context-evidence-capture.md)

Goal: capture the app, ownership, operational, and business context needed to
interpret the machine findings. This step keeps the workflow from guessing what
the database signals mean inside the company.
Agents may gather semi-structured sources and draft a context pack; humans
validate the facts before they are used for diagnosis.

Work:

- `[agent]` gather source artifacts and operator commentary tied to the machine
  findings
- `[agent]` draft a context pack with candidate facts, ownership mappings,
  source references, and open questions
- `[human]` validate ownership, doc freshness, business criticality, safe
  throttles, and approval boundaries
- `[human]` confirm which context unknowns could change the diagnosis or
  recommendation

Exit artifact:

- context pack
- ownership map
- validated timeline
- known-noise and safe-action notes
- context unknowns that could change the recommendation

### 3. Company-Aware Recommendation

Deep dive: [Company-aware recommendation](deep-dives/company-aware-recommendation.md)

Goal: turn machine findings and company context into the recommendation brief.
This step contains the diagnosis, confidence, tradeoffs, actions to take,
actions to avoid, and follow-up work.

Work:

- `[agent]` join findings with validated timelines and context to draft
  priority, assumptions, unknowns, and falsification checks
- `[agent]` frame candidate actions by mitigation, repair, follow-up, actions
  to avoid, owner, reversibility, risk, and approval boundary
- `[agent]` draft the incident recommendation brief and follow-up tickets
- `[human]` validate priority, causal chain, confidence, action path, and any
  production-impacting change

Exit artifact:

- incident recommendation brief
- action path, actions to avoid, and follow-up tickets
- confidence and approval-boundary notes

## Target Timeline

The target is a few hours when the design partner can provide the evidence
package quickly. This remains a hypothesis until design-partner trials show the
evidence and context work can fit inside the window.

| Time | Activity | Output |
| --- | --- | --- |
| T+0 to T+30m | Intake and scope gate | Intake state, evidence readiness, first branch selected |
| T+30m to T+150m | Machine evidence and analysis | Evidence manifest, ranked findings, missing evidence |
| T+150m to T+210m | Context evidence capture | Context pack, ownership map, validated timeline |
| T+210m to T+330m | Company-aware recommendation | Recommendation brief with diagnosis and action path |
| T+330m to T+360m | Customer review | Revised final brief |

One day remains the outer bound for cases with slower data access, multiple
possible causal chains, or missing ownership context.

## Engagement Rejection Criteria

Reject or re-scope the engagement when:

- the customer cannot provide logs or metrics for the relevant window
- there is no operator available to answer context questions
- the request is a generic health check disguised as an incident
- the expected output is guaranteed root cause without sufficient evidence
- the customer wants autonomous production changes
- the issue is clearly a basic configuration or education problem
