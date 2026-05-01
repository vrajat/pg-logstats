# Context Evidence Capture

Status: living deep dive  
Parent: [Successful engagement criteria, workflow, and timeline](../successful-engagement.md)

## Question

How should the engagement gather and structure the application, ownership,
operational, and business context needed to interpret machine findings?

## Current Framing

Context evidence capture is where agents become materially useful.

The work is not just asking an expert questions. The agent can gather PDFs,
docs, links, runbooks, incident notes, diagrams, Confluence pages, ticket
threads, and operator commentary, then process that semi-structured information
into a reviewable context pack.

The exact standard form is not yet decided. The near-term goal is to make the
context concrete, source-linked, and easy for operators to correct.

## Inputs To Gather

The agent-assisted workflow should request and organize:

- Confluence, Notion, Google Docs, or internal wiki links
- PDFs, architecture diagrams, and runbooks
- incident review docs and RCA notes
- Jira, Linear, GitHub, or deployment tickets
- deploy, migration, and feature-flag timelines
- service catalog or ownership data
- dashboards or dashboard screenshots
- Slack or incident-channel excerpts where shareable
- database user, `application_name`, service, job, and team mappings
- operator walkthrough notes, voice notes, or typed commentary

If auth is required, the workflow should ask for the narrowest practical access
or request customer-side exports.

## Agent Workflow Opportunity

The agent should gather, normalize, and prepare context for human review.

Useful agent work:

- ask for the right docs and links based on the machine findings
- request auth, exports, or screenshots when direct access is not possible
- crawl or ingest approved docs
- extract candidate facts with source references
- identify stale, conflicting, or unsupported claims
- map services, jobs, database users, and `application_name` values to owners
- turn operator commentary into structured notes
- generate targeted follow-up questions
- keep an unknowns list for facts that could change diagnosis or mitigation

The agent should not silently promote extracted claims to truth. It should mark
claims for operator validation.

## Human Review Boundary

Operators should validate the context pack.

Human review is required for:

- confirming service ownership
- confirming whether docs are current
- correcting topology assumptions
- identifying business-critical paths
- approving use of incident-channel excerpts
- validating safe throttles and unsafe actions
- confirming which context gaps matter for the current incident

The human role should be correction and validation, not blank-page explanation.

## Possible Standardization Shape

The standard form should probably be small and practical at first.

Candidate sections:

- topology facts
- ownership map
- workload map
- deployment and migration timeline
- business criticality map
- known noisy or benign workloads
- safe mitigation boundaries
- stale or contradicted docs
- context unknowns
- source references

The important property is not the final schema. The important property is that
each fact has a source, freshness signal, confidence label, and owner when
available.

## Example 1: Query Family To Service Ownership

Machine finding:

- `application_name = checkout-api`
- database user `app_checkout`
- query family writes to `orders`, `discounts`, and `cart_items`
- latency spike starts after a deploy

Agent-gathered context:

- service catalog says `checkout-api` is owned by Payments Platform
- Confluence checkout runbook says discount enrichment can be disabled by flag
- deploy ticket says `discount-preload-v2` shipped 20 minutes before the spike
- incident notes say large enterprise carts are high revenue and should not be
  globally disabled without approval

Reviewable context pack entry:

- owner: Payments Platform
- product path: checkout
- suspected deploy: `discount-preload-v2`
- safe mitigation candidate: disable discount enrichment for large carts only
- approval boundary: checkout degradation requires incident commander approval
- confidence: needs operator confirmation
- sources: service catalog, runbook, deploy ticket, incident note

## Example 2: Replica Lag And CDC Impact

Machine finding:

- replica lag increases during incident window
- CDC slot lag grows
- primary write volume and WAL rate rise
- read latency increases on a reporting replica

Agent-gathered context:

- architecture doc says analytics reads should use the reporting replica
- data pipeline docs say CDC feeds billing reconciliation
- runbook says billing can tolerate 30 minutes of lag but support dashboards
  cannot tolerate stale account status
- operator walkthrough says replica lag alerts are often secondary during
  nightly exports

Reviewable context pack entry:

- affected systems: analytics replica, CDC billing feed, support dashboard
- likely noisy workload: nightly export
- business impact split: billing can lag briefly; support account status is
  customer-facing
- context question: did export concurrency change today?
- confidence: mixed; needs timeline validation
- sources: architecture doc, data pipeline doc, runbook, operator walkthrough

## Output Artifact

The context evidence artifact should include:

- context pack
- ownership map
- workload map
- validated timeline
- known-noise notes
- safe-action notes
- approval-boundary notes
- stale-doc list
- contradicted-claim list
- context unknowns
- source references

## Agent Step Risk Analysis

Scores:

- `No risk`: bounded, checkable, structured, and verifiable enough for agent
  acceleration with normal human review.
- `Risk`: useful, but can bias the engagement if not reviewed or constrained.
- `Unknown`: not enough examples yet to decide whether the agent role is
  reliable.

| Agent step | Score | Why |
| --- | --- | --- |
| Request relevant docs, PDFs, links, exports, screenshots, or narrow auth | Risk | The agent can over-request sensitive material; requests must be scoped to the incident and approved. |
| Ingest approved docs and commentary | No risk | Ingestion is bounded if sources are explicitly approved and tracked. |
| Extract candidate facts and label confidence or freshness | Risk | Extraction can be wrong or stale; claims must remain source-linked and operator-validated. |
| Collect operator commentary through walkthroughs, voice notes, or typed corrections | No risk | The agent is structuring human-provided context, not deciding truth. |
| Map `application_name`, database users, service names, jobs, and query families to owners and product paths | Unknown | This depends on local naming quality and service catalogs; design-partner examples are needed. |
| Draft a small context pack with source references and open questions | No risk | The artifact is reviewable if every claim has a source and validation status. |

Information that would improve the scores:

- example context packs from historical incidents
- a small tentative claim schema
- source-linking requirements
- freshness labels for docs
- operator correction logs
- examples of claims that changed diagnosis or mitigation

## Working Thesis

Agents can materially speed context evidence capture because most of the work is
semi-structured gathering, extraction, contradiction detection, and targeted
follow-up generation.

The human value is validation. The operator should correct a draft model of the
environment rather than write the model from scratch.
