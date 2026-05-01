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

### 0. Intake And Triage

Deep dive: [Intake and triage](deep-dives/intake-and-triage.md)

Goal: decide whether the case is eligible for a high-speed engagement.
This step prevents wasting the engagement on cases that lack severity, evidence,
or access to the people who can validate context.

Work:

- review OSS-generated findings or machine-evidence summaries when available
- identify the incident class: latency, saturation, replication/CDC, storage,
  failover, migration, cost, or unknown
- confirm production scale and severity
- confirm machine-evidence readiness: logs, stats, metrics, baselines, and
  parseability
- confirm context-evidence readiness: owners, deploy history, runbooks,
  business impact, and operator availability
- identify any active safety constraints
- choose the first investigation branch
- decide whether the case is ready, needs machine evidence, needs context
  evidence, or should be rejected/re-scoped

Exit artifact:

- one-page intake brief
- machine-evidence readiness score
- context-evidence readiness score
- initial hypothesis map
- intake state: `Ready`, `Needs machine evidence`, `Needs context evidence`, or
  `Reject / re-scope`

### 1. Machine Evidence Assembly

Goal: make the machine side of the problem bounded, checkable, structured, and
reviewable. This step creates the raw factual base for deterministic analysis.

Work:

- collect logs and metrics into a shared evidence folder
- normalize timezones and incident windows
- separate target and baseline windows
- build topology notes for primary, replicas, CDC, poolers, and major clients
- identify missing or low-trust signals

Exit artifact:

- evidence manifest
- timeline v0
- topology v0

### 2. Deterministic Analysis Of Machine Evidence

Goal: compress raw evidence into ranked findings.
This step turns high-volume machine data into a small set of findings that can
be inspected, challenged, and joined with context.

Work:

- rank query families by total time, count, max, p95, and change from baseline
- identify lock waits, temp files, errors, autovacuum/checkpoint signals, and
  replication-related warnings
- correlate query families with database, user, `application_name`, process,
  client, and time window
- compare primary and replica symptoms where data exists
- produce suggested follow-up SQL and system-view checks

`pg-logstats` can be one tool in this phase, especially for compact log-derived
findings. It should not be the only evidence path.

Exit artifact:

- ranked evidence table
- finding IDs with source references
- unanswered-data list

### 3. Context Evidence Capture

Goal: capture the app, ownership, operational, and business context needed to
interpret the machine findings. This step keeps the workflow from guessing what
the database signals mean inside the company.

Work:

- ingest runbooks, incident notes, topology docs, deploy timelines, and service
  ownership docs
- extract candidate facts and mark them confirmed, likely, stale, contradicted,
  or unknown
- collect operator commentary through a walkthrough, voice notes, or typed
  corrections
- map `application_name`, database users, service names, jobs, and query
  families to owners and product paths
- capture known noisy workloads, safe throttles, business priorities, and
  approval boundaries

Exit artifact:

- context pack
- ownership map
- validated timeline
- known-noise and safe-action notes
- context unknowns that could change the recommendation

### 4. Company-Aware Diagnosis

Goal: translate database evidence into application and operational meaning.
This step joins machine evidence with context evidence to produce the causal
chain and confidence level.

Work:

- map findings to services, jobs, tenants, endpoints, product workflows, or data
  pipelines
- compare against deploy, migration, feature-flag, maintenance, and bulk-job
  timelines
- identify known-benign noise and exclude it only when reversible and explicit
- choose the most likely causal chain
- identify what would falsify the diagnosis

Exit artifact:

- diagnosis memo
- causal chain
- confidence and falsification notes

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
