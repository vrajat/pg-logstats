# Database Operations Service Engagement

Status: living draft  
Date: 2026-05-01

## Goal

Define a professional service engagement that can deliver high-value database
operations recommendations for a production PostgreSQL installation in a few
hours, with one day as the outer bound.

This document uses `pg-logstats` as a concrete trial vehicle, but the target
category is broader: production database operations for systems where SLA, cost,
and uptime are at risk. The lessons should transfer to Postgres-backed
applications, data integration systems, analytics stacks, and high-throughput
key-value or event stores.

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

## Section 1: Criteria, Workflow, And Timeline For A Successful Engagement

### Success Criteria

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

### Non-Goals

The engagement should not optimize for:

- replacing the customer's incident commander
- autonomously changing production
- producing a comprehensive health audit
- teaching Postgres basics
- building dashboards
- recreating pgBadger as a prettier report
- making `pg-logstats` the center of the story

### Evidence Required Before Diagnosis

The minimum evidence package should include:

- incident timeline with alert times, customer impact, and mitigation attempts
- PostgreSQL version, hosting model, topology, and failover mechanism
- primary and replica roles, read-routing rules, and CDC topology
- relevant Postgres logs for the incident window and nearby baseline windows
- `pg_stat_statements` snapshot or export when available
- active session and lock snapshots if captured
- replica lag, WAL, checkpoint, vacuum, and IO metrics
- connection pool metrics
- deploy, migration, feature-flag, and bulk-job timeline
- application ownership map for major query families or service names
- current runbooks and recent similar incidents

The evidence package must distinguish observed facts from operator guesses.

### Engagement Workflow

#### 0. Intake And Triage

Goal: decide whether the case is eligible for a high-speed engagement.

Work:

- identify the incident class: latency, saturation, replication/CDC, storage,
  failover, migration, cost, or unknown
- confirm production scale and severity
- confirm access to logs, metrics, and operators
- identify any active safety constraints
- choose the first investigation branch

Exit artifact:

- one-page intake brief
- evidence checklist
- initial hypothesis map

#### 1. Evidence Assembly

Goal: make the problem bounded, checkable, structured, and reviewable.

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

#### 2. Deterministic Analysis

Goal: compress raw evidence into ranked findings.

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

#### 3. Company-Aware Diagnosis

Goal: translate database evidence into application and operational meaning.

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

#### 4. Recommendation And Decision Framing

Goal: convert diagnosis into high-value action guidance.

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

### Target Timeline

The target is a few hours when the design partner can provide the evidence
package quickly.

| Time | Activity | Output |
| --- | --- | --- |
| T+0 to T+30m | Intake and scope gate | Case accepted or rejected, first branch selected |
| T+30m to T+90m | Evidence assembly | Evidence manifest, timeline v0, topology v0 |
| T+90m to T+150m | Deterministic analysis | Ranked findings and missing evidence |
| T+150m to T+210m | Diagnosis | Causal chain and confidence |
| T+210m to T+270m | Recommendations | Mitigation and follow-up brief |
| T+270m to T+360m | Customer review | Revised final brief |

One day remains the outer bound for cases with slower data access, multiple
possible causal chains, or missing ownership context.

### Engagement Rejection Criteria

Reject or re-scope the engagement when:

- the customer cannot provide logs or metrics for the relevant window
- there is no operator available to answer context questions
- the request is a generic health check disguised as an incident
- the expected output is guaranteed root cause without sufficient evidence
- the customer wants autonomous production changes
- the issue is clearly a basic configuration or education problem

## Section 2: Workflow To Request From A Design Partner

### Design Partner Goal

The design partner should help us discover the real operational workflow, not
validate a prebuilt tool demo.

The goal is to observe how an expert team currently investigates difficult
database incidents, then identify where a fast evidence-plus-agent workflow can
compress time-to-understanding and time-to-decision.

### Design Partner Profile

Strong candidates have:

- production Postgres at meaningful scale
- recurring SLA, cost, uptime, replica, CDC, or query-performance incidents
- a team that has already tried internal diagnosis
- access to historical incident artifacts
- willingness to share anonymized logs, metrics, timelines, and runbooks
- named operators who can walk through their real investigation process

Weak candidates have:

- only toy workloads
- only greenfield observability interests
- no incident history
- no access to raw evidence
- desire for a generic dashboard or report

### Requested Workflow

#### 1. Pre-Engagement Audit

Ask the design partner to pick one or two recent incidents where the internal
team struggled.

Request:

- incident summary
- incident timeline
- production topology
- logs and metrics for incident and baseline windows
- relevant deploy, migration, feature-flag, or job timeline
- runbooks used during the incident
- final internal RCA or unresolved questions
- current pain points in the investigation workflow

The audit should include a live walkthrough with the people who did the work.
The point is to capture the actual workflow, including undocumented steps,
Slack queries, dashboards, shell commands, and judgment calls.

#### 2. Evidence Package Trial

Ask the partner to provide a sanitized evidence package for one incident.

The package should be complete enough that an external investigator can work
without broad production access:

- logs
- metrics exports or screenshots with timestamps
- SQL snapshots where available
- topology notes
- application ownership notes
- known noise filters
- incident channel transcript or summarized decision log

Measure how long it takes the partner to assemble this package. That assembly
time is part of the product and service design problem.

#### 3. Timed Investigation Trial

Run a timed investigation against the evidence package.

Rules:

- no production changes
- human approval for any live SQL
- all claims tied to evidence
- recommendations separated into immediate, short-term, and structural
- unknowns called out explicitly

Target output within a few hours:

- ranked findings
- likely causal chain
- action recommendation
- data that would change the recommendation
- follow-up instrumentation or policy suggestions

#### 4. Review Against Ground Truth

Review the output with the design partner.

Ask:

- did the workflow identify the real issue faster than the team did?
- did it surface anything the team missed?
- were the recommendations too generic or actually actionable?
- did it correctly avoid unsafe actions?
- what internal context was required to make the answer useful?
- what part of the evidence package was hardest to assemble?
- what would make this trustworthy during a live incident?

#### 5. Repeat On A Live Or Near-Live Case

After one historical trial, repeat on a fresh incident or near-live degradation.

The goal is to test:

- speed under pressure
- data access friction
- operator trust
- quality of recommendations before the team knows the answer
- usefulness of the final artifact in an incident channel

### What We Need To Learn From The Design Partner

- Which incident classes are painful enough to pay for?
- Which evidence is always available, sometimes available, or usually missing?
- Which recommendations are considered high value by senior operators?
- Which actions require explicit approval boundaries?
- Which company-specific context changes the diagnosis?
- How much of the workflow can be deterministic?
- Where does model judgment actually belong?
- What artifact format is most useful: incident brief, action tree, RCA addendum,
  runbook patch, or ticket list?
- How fast can evidence be safely packaged?

### Initial Trial Hypotheses

- The highest-value service is not log reporting; it is rapid diagnosis plus
  decision framing.
- The first durable product primitive is an evidence package, not an agent UI.
- `pg-logstats` is useful if it creates compact, stable, machine-readable
  findings that feed the engagement.
- The workflow should be mostly deterministic until the diagnosis and
  recommendation stage.
- Company context matters most for ownership, noise suppression, business
  impact, and mitigation choice.
- A design partner will reveal that evidence assembly is a major bottleneck.

## External Inputs Reviewed

- pgBadger documentation: <https://access.crunchydata.com/documentation/pgbadger/latest/>
- pgBadger incremental reports announcement:
  <https://www.postgresql.org/about/news/pgbadger-5-analyze-your-logs-daily-with-the-incremental-mode-1505/>
- Community pgBadger usage recap:
  <https://techcommunity.microsoft.com/blog/adforpostgresql/community-insights-on-pgbadger-a-pgsql-phriday-010-recap/3880911>
- pgBadger issue on hourly RDS incremental reports:
  <https://github.com/darold/pgbadger/issues/697>
- GitLab incident: statement timeouts and query planner statistics:
  <https://gitlab.com/gitlab-com/gl-infra/production/-/issues/3875>
- GitLab incident review: replica and primary saturation from expensive query
  traffic:
  <https://gitlab.com/gitlab-com/gl-infra/production/-/issues/20692>
- PostgreSQL mailing list incident: large table, autovacuum, and explosive
  replication lag:
  <https://www.postgresql.org/message-id/CA%2BB3G4QvCy_a2%2BSUbbeQU5Q0TzUrH2iUYKg-XBezeJDMfVoq7Q%40mail.gmail.com>
- PostgreSQL monitoring documentation:
  <https://www.postgresql.org/docs/current/monitoring.html>
- PostgreSQL CDC failure modes:
  <https://streams.dbconvert.com/blog/postgresql-cdc-breaks-in-production/>

## Open Questions

- Should the first design partner trial focus on a historical incident or a
  current recurring degradation?
- Which incident class should be first: slow queries, replica lag, CDC
  correctness, storage growth, or cost?
- What is the smallest evidence package that can still support principal-level
  recommendations?
- Should the final service artifact be optimized for incident command, executive
  communication, or engineering follow-up?
- What information can be safely requested from customers without requiring
  privileged production access?
