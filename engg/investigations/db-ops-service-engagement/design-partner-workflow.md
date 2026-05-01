# Design Partner Workflow

Status: living investigation  
Parent: [Database Operations Service Engagement](README.md)

## Design Partner Goal

The design partner should help us discover the real operational workflow, not
validate a prebuilt tool demo.

The goal is to observe how an expert team currently investigates difficult
database incidents, then identify where a fast evidence-plus-agent workflow can
compress time-to-understanding and time-to-decision.

## Design Partner Profile

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

## Requested Workflow

### 1. Pre-Engagement Audit

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

### 2. Evidence Package Trial

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

### 3. Timed Investigation Trial

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

### 4. Review Against Ground Truth

Review the output with the design partner.

Ask:

- did the workflow identify the real issue faster than the team did?
- did it surface anything the team missed?
- were the recommendations too generic or actually actionable?
- did it correctly avoid unsafe actions?
- what internal context was required to make the answer useful?
- what part of the evidence package was hardest to assemble?
- what would make this trustworthy during a live incident?

### 5. Repeat On A Live Or Near-Live Case

After one historical trial, repeat on a fresh incident or near-live degradation.

The goal is to test:

- speed under pressure
- data access friction
- operator trust
- quality of recommendations before the team knows the answer
- usefulness of the final artifact in an incident channel

## What We Need To Learn From The Design Partner

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

## Initial Trial Hypotheses

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
