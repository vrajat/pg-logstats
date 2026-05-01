# Successful Engagement Criteria, Workflow, And Timeline

Status: living investigation  
Parent: [Database Operations Service Engagement](README.md)

## Goal

Define the criteria, workflow, and timeline for a professional service
engagement that can deliver high-value production database operations
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

The target environment is not a small application database. Assume:

- terabytes of data
- many tens of thousands of queries per second
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

Success means high-value recommendations in record time.

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

- a ranked diagnosis tied to specific evidence
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

### 0. Intake And Triage

Deep dive: [Intake and triage](deep-dives/intake-and-triage.md)

Goal: decide whether the case is eligible for a high-speed engagement.
This step prevents wasting the engagement on cases that lack severity, evidence,
or access to the people who can validate context.

Work:

- `[script]` review OSS-generated findings or machine-evidence summaries when
  available
- `[agent]` summarize customer notes and machine findings into a structured
  intake brief
- `[agent]` identify the likely incident class: latency, saturation,
  replication/CDC, storage, failover, migration, cost, or unknown
- `[human]` confirm production scale, severity, and service fit
- `[script]` confirm machine-evidence readiness: logs, stats, metrics, baselines, and
  parseability
- `[agent]` detect contradictions or gaps between the customer narrative and
  the machine-evidence packet
- `[human]` confirm context-evidence readiness: owners, deploy history, runbooks,
  business impact, and operator availability
- `[human]` identify any active safety constraints
- `[agent]` suggest the first investigation branch
- `[human]` decide whether the case is ready, needs machine evidence, needs context
  evidence, or should be rejected/re-scoped

Exit artifact:

- one-page intake brief
- machine-evidence readiness score
- context-evidence readiness score
- initial hypothesis map
- intake state: `Ready`, `Needs machine evidence`, `Needs context evidence`, or
  `Reject / re-scope`

### 1. Machine Evidence Assembly

Deep dive: [Machine evidence assembly](deep-dives/machine-evidence-assembly.md)

Goal: make the machine side of the problem bounded, checkable, structured, and
reviewable. This step creates the raw factual base for deterministic analysis.
It should be deterministic collection with agent-guided walkthrough support for
exceptions, provider-specific paths, redaction choices, and missing artifacts.

Work:

- `[agent]` guide the operator through the correct collection recipe for the
  provider, topology, and available data sources
- `[script]` collect logs and metrics into a shared evidence folder
- `[script]` normalize timezones and incident windows
- `[script]` separate target and baseline windows
- `[agent]` suggest fallback baseline windows when none were provided
- `[script]` build topology notes for primary, replicas, CDC, poolers, and major
  clients from collected metadata where possible
- `[human]` approve data sharing, live read-only SQL, and redaction boundaries
- `[script]` apply and record redaction policy
- `[script]` validate parseability and completeness
- `[agent]` explain missing or low-trust signals and route context gaps to
  context evidence capture

Exit artifact:

- evidence manifest
- timeline v0
- topology v0
- redaction report
- parseability and completeness report

### 2. Deterministic Analysis Of Machine Evidence

Deep dive: [Deterministic analysis of machine evidence](deep-dives/deterministic-analysis-of-machine-evidence.md)

Goal: compress raw evidence into ranked findings.
This step turns high-volume machine data into a small set of findings that can
be inspected, challenged, and joined with context.
The scripts produce the findings; the agent drives the tool loop and prepares a
walkthrough; humans review the artifact before it feeds diagnosis.

Work:

- `[agent]` choose which deterministic tools and windows to run from the
  evidence bundle
- `[script]` rank query families by total time, count, max, p95, and change from
  baseline
- `[script]` identify lock waits, temp files, errors, autovacuum/checkpoint
  signals, replication warnings, CDC lag, and pool saturation
- `[script]` correlate findings with database, user, `application_name`,
  process, client, replica, and time window
- `[agent]` compare outputs across baseline windows and adjust thresholds when
  the first pass is too noisy or empty
- `[script]` generate stable finding IDs, source references, and machine-readable
  output
- `[agent]` prepare a findings walkthrough and separate findings from hypotheses
- `[human]` review the artifact for credibility, collection artifacts, and live
  SQL safety
- `[agent]` route missing evidence and context questions to the next workflow
  step

`pg-logstats` can be one tool in this phase, especially for compact log-derived
findings. It should not be the only evidence path.

Exit artifact:

- ranked evidence table
- finding IDs with source references
- analysis windows, thresholds, and filters used
- unanswered-data list
- suggested follow-up SQL
- context questions generated from machine findings

### 3. Context Evidence Capture

Deep dive: [Context evidence capture](deep-dives/context-evidence-capture.md)

Goal: capture the app, ownership, operational, and business context needed to
interpret the machine findings. This step keeps the workflow from guessing what
the database signals mean inside the company.
Agents gather semi-structured sources and draft a context pack; humans validate
the facts before they are used for diagnosis.

Work:

- `[agent]` request relevant docs, PDFs, links, exports, screenshots, or narrow
  auth based on the machine findings
- `[agent]` ingest runbooks, incident notes, topology docs, deploy timelines,
  service ownership docs, and operator commentary
- `[agent]` extract candidate facts and mark them confirmed, likely, stale,
  contradicted, or unknown
- `[agent]` collect operator commentary through a walkthrough, voice notes, or
  typed corrections
- `[agent]` map `application_name`, database users, service names, jobs, and query
  families to owners and product paths
- `[agent]` draft a small context pack with source references and open questions
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

### 4. Company-Aware Diagnosis

Deep dive: [Company-aware diagnosis](deep-dives/company-aware-diagnosis.md)

Goal: translate database evidence into application and operational meaning.
This step joins machine evidence with context evidence to produce the causal
chain and confidence level.
Scripts provide the machine diagnosis; agents join and prioritize against
context evidence; humans validate the weighting and causal interpretation.

Work:

- `[script]` provide machine-ranked findings, severity metrics, deltas, source
  references, and evidence confidence
- `[agent]` map findings to services, jobs, tenants, endpoints, product workflows, or data
  pipelines
- `[agent]` compare against deploy, migration, feature-flag, maintenance, and bulk-job
  timelines
- `[agent]` identify likely secondary effects and known-benign noise, but keep
  exclusions explicit and reversible
- `[agent]` propose a priority order that weighs machine severity against
  business impact, ownership, known noise, and timing
- `[agent]` identify assumptions, unknowns, and falsification checks for each
  candidate diagnosis
- `[human]` validate whether the weighting matches operator knowledge and
  business priorities
- `[human]` approve the causal chain and confidence level before recommendations

Exit artifact:

- prioritized diagnosis memo
- causal chain
- machine and context evidence summary
- confidence and falsification notes
- operator validation notes

### 5. Recommendation And Decision Framing

Goal: convert diagnosis into high-value action guidance.
This step turns the insight into decisions: what to do now, what to defer, what
to avoid, and what to change permanently.

Work:

- separate immediate mitigation from root-cause repair
- call out action reversibility, blast radius, and likely time-to-effect
- identify actions to avoid
- identify owners and coordination path
- recommend follow-up instrumentation, runbook, policy, or architecture changes

Exit artifact:

- incident recommendation brief
- action tree
- owner map
- follow-up ticket list

## Target Timeline

The target is a few hours when the design partner can provide the evidence
package quickly.

| Time | Activity | Output |
| --- | --- | --- |
| T+0 to T+30m | Intake and scope gate | Intake state, evidence readiness, first branch selected |
| T+30m to T+90m | Machine evidence assembly | Evidence manifest, timeline v0, topology v0 |
| T+90m to T+150m | Deterministic analysis | Ranked findings and missing evidence |
| T+150m to T+210m | Context evidence capture | Context pack, ownership map, validated timeline |
| T+210m to T+270m | Diagnosis | Causal chain and confidence |
| T+270m to T+330m | Recommendations | Mitigation and follow-up brief |
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
