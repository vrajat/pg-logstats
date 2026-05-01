# Evidence Required Before Diagnosis

Status: living deep dive  
Parent: [Successful engagement criteria, workflow, and timeline](../successful-engagement.md)

## Question

What evidence is required before a high-speed database operations service can
make useful recommendations, and how can the collection process be made fast,
low-friction, and high-fidelity?

## Current Framing

There are two evidence categories:

1. Machine evidence: logs, metrics, SQL snapshots, statistics views, pooler
   state, replication state, and outputs from utilities like `pg-logstats` or
   pgBadger.
2. Context evidence: applications, service ownership, deploy history, runbooks,
   undocumented operator knowledge, customer impact, business criticality, and
   prior incident memory.

The diagnosis quality comes from joining the two. Machine evidence can say that
a query family, replica lag, WAL rate, lock wait, or pool saturation changed.
Context evidence explains which product path it belongs to, who owns it, what
changed recently, whether it is expected, and which mitigations are safe.

## Machine Evidence

### Problem

The main friction is access.

The customer may hesitate to share production logs, metrics, query text,
connection details, tenant identifiers, internal hostnames, or application
metadata. Even when they are willing, collecting the data may require multiple
systems and teams.

This part should not require a heavy professional services engagement. It
should be productized into a self-serve evidence-bundle path as much as
possible.

### Mitigations

`pg-logstats` is open source, so the preferred prior action is for the customer
to install it, run it locally, and generate findings without granting external
access to raw production logs.

Additional mitigations:

- provide a questionnaire or walkthrough that goes beyond open-source docs
- offer a single offline evidence-bundle command
- avoid requiring production write access
- use read-only SQL and offline logs where possible
- redact literals, tenant IDs, emails, IPs, and optionally table names
- hash sensitive identifiers while keeping stable grouping keys
- produce a completeness score for the evidence package
- include sanity checks for log prefix, sparse logging, missing `queryid`,
  missing `application_name`, clock skew, and unusable baselines
- provide cloud-provider recipes for RDS, Aurora, Cloud SQL, self-hosted, and
  Kubernetes deployments
- support incident, previous-window, same-hour-yesterday, and same-day-last-week
  windows
- generate stable JSON findings that can be shared without raw logs

### Candidate Bundle Contents

The evidence bundle should include:

- normalized incident window metadata
- baseline window metadata
- log-derived findings
- `pg_stat_statements` export when available
- active session snapshots if captured
- lock snapshots if captured
- `pg_stat_replication`, `pg_stat_replication_slots`, and WAL-related state
- checkpoint, autovacuum, IO, and table activity snapshots
- connection pool metrics or snapshots
- collector version and configuration
- redaction policy used
- missing-evidence report

### Product Implication

Machine evidence should become a product primitive, not a consulting bottleneck.

The service engagement should ideally begin after the customer has produced an
evidence bundle. If setup itself is hard, that is a product-readiness problem to
solve with documentation, scripts, and packaging.

## Context Evidence

### Problem

This is the high-friction part.

Even when a leader wants the engagement, the expert or lead operator may not
want to spend time on it. They may be busy, skeptical, under incident pressure,
or worried that the process will turn into generic consulting. They may also
miss important context because the knowledge is habitual and not written down.

The risk is not merely missing documents. The decisive context often lives in:

- Slack threads
- dashboard habits
- shell history
- incident muscle memory
- ownership conventions
- recent deploy lore
- stale runbooks with local corrections
- known noisy jobs
- implicit business priority rules

### Desired Approach

Do not ask the expert to "provide context" from scratch.

Instead, ask the expert to review a draft model of their world. People are
faster and more accurate when correcting concrete claims than when filling out a
blank questionnaire.

### High-Fidelity Collection Methods

Use a deterministic walkthrough that lets the operator upload and review docs,
enter commentary, and respond by voice or typing.

The workflow should:

- ingest runbooks, incident docs, architecture notes, dashboards, and deploy
  notes
- extract factual claims with source links
- label claims as confirmed, likely, stale, contradicted, or unknown
- generate follow-up questions from gaps and contradictions
- build an ownership map from `application_name`, database users, service names,
  deploy history, docs, and human corrections
- ask forced-ranking questions instead of broad free-form questions
- capture operator heuristics explicitly
- maintain an unknowns board for facts that could change the recommendation
- keep a reusable customer context pack updated after each engagement

### Walkthrough Shape

A useful walkthrough could proceed as:

1. Upload docs, diagrams, incident links, runbooks, and exported dashboards.
2. System extracts claims and builds a first-pass topology, ownership map, and
   runbook index.
3. Operator reviews the draft model and corrects wrong or stale claims.
4. System asks targeted follow-ups only where the answer changes diagnosis or
   mitigation.
5. Operator narrates a 30-minute screen-share or voice walkthrough of how they
   would investigate this class of incident.
6. System converts the narration into operator heuristics, dashboard sequence,
   decision points, and missing-data questions.
7. Final context pack is produced with confidence labels and source references.

### Useful Question Types

Prefer questions that are easy to answer under time pressure:

- Which services can hurt revenue fastest?
- Which jobs are allowed to lag?
- Which workloads are safe to throttle?
- Which apps are known noisy but benign?
- Which replicas serve user-facing reads?
- Which metrics do you check first for this alert?
- Which runbook is stale?
- Which team owns this `application_name`?
- Which recent deploys, migrations, or jobs are suspicious?
- Which action would be unacceptable without executive approval?

Avoid broad prompts like:

- Describe your architecture.
- Tell us all relevant context.
- What should we know?

## Joining Machine And Context Evidence

The evidence process should explicitly join the two categories:

- query family to service or endpoint
- database user to owning system
- `application_name` to team
- replica lag to read-routing behavior
- CDC lag to downstream data correctness impact
- pool saturation to app traffic class
- deploy timeline to symptom onset
- known maintenance job to noisy log findings
- business priority to mitigation ordering

This join is where high-value recommendations emerge.

## What Else May Be Needed

Additional mechanisms worth exploring:

- a customer-side evidence agent that runs continuously but only exports
  approved bundles
- pre-signed upload with local redaction and manifest review
- a "minimum viable evidence" gate before the service clock starts
- synthetic incident rehearsals to test whether evidence can be collected fast
- reusable customer context packs with explicit freshness dates
- post-engagement patches to runbooks and evidence collectors
- customer-specific suppression rules that are reversible and auditable
- service-level agreements for human context availability during the engagement

## Working Thesis

Machine evidence can be made mostly self-serve. Context evidence is the real
friction and likely the real service moat.

The fastest path is not to interview experts from a blank page. It is to build a
draft operational model from artifacts and machine evidence, then ask experts to
correct, rank, and narrate the parts that matter.
