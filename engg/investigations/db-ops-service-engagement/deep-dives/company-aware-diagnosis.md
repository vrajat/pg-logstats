# Company-Aware Diagnosis

Status: living deep dive  
Parent: [Successful engagement criteria, workflow, and timeline](../successful-engagement.md)

## Question

How should the engagement combine deterministic machine findings with company
context to produce a prioritized diagnosis?

## Current Framing

Company-aware diagnosis is a join and prioritization loop.

The machine diagnosis is script-owned: what changed, where, by how much, and
with what evidence. The company-aware diagnosis combines that with context
evidence: ownership, product impact, business criticality, deploy history,
known-noise patterns, and safe operating boundaries.

The weighting between machine severity and company importance is fuzzy. It
requires agent-assisted synthesis and human validation.

## Inputs

Machine diagnosis inputs:

- ranked machine findings
- finding IDs and source references
- time windows and baseline comparisons
- severity metrics: count, total time, p95, max, lag, wait time, error rate,
  saturation, or cost proxy
- confidence in machine evidence
- missing or low-quality machine inputs
- suggested follow-up SQL

Context inputs:

- ownership map
- workload map
- deploy, migration, job, and feature-flag timeline
- business criticality map
- known noisy or benign workloads
- safe mitigation boundaries
- stale or contradicted docs
- context unknowns

## What Should Stay Script-Owned

Scripts should retain responsibility for machine scoring:

- raw severity
- deltas from baseline
- rank among machine findings
- stable grouping
- source references
- reproducible thresholds and filters
- evidence confidence based on data quality

The script can say a query family is expensive, new, or sharply worse. It
should not decide that the query matters more than another finding because it
belongs to checkout, billing, or an executive dashboard.

## Agent Workflow Opportunity

The agent should perform the first synthesis pass.

Useful agent work:

- join machine findings to services, jobs, tenants, endpoints, product paths, or
  data pipelines
- compare symptom onset against deploys, migrations, feature flags, and jobs
- propose candidate causal chains
- identify findings that are likely secondary effects
- identify findings that are likely known-benign noise
- explain why machine severity and business priority disagree
- generate clarifying questions for operators
- propose a diagnosis priority order with explicit assumptions
- identify what would falsify each candidate diagnosis

The agent should make its weighting visible rather than hide it behind a single
score.

## Human Review Boundary

Humans should validate the weighting and causal interpretation.

Human review is needed for:

- whether a product path is actually business-critical
- whether a noisy workload is safe to discount
- whether a deploy correlation is meaningful or coincidental
- whether a low-severity database signal is high-priority because of customer
  impact
- whether a high-severity machine finding is low-priority because it is expected
  maintenance
- whether the proposed causal chain matches operator experience
- whether the confidence level is honest

The human role is not to re-run the analysis. It is to validate the judgment
layer that sits on top of deterministic evidence.

## Weighting Loop

The diagnosis loop should be explicit:

1. Start with machine-ranked findings.
2. Join each finding to context evidence.
3. Assign a provisional priority using both machine severity and context impact.
4. Mark assumptions and unknowns.
5. Ask targeted operator questions only where answers could change priority or
   causal chain.
6. Re-rank after operator validation.
7. Produce a diagnosis memo with confidence and falsification notes.

This loop is necessary because the right answer may not be the highest machine
severity finding.

## Example: High Machine Severity, Lower Business Priority

Machine finding:

- export worker query family dominates total runtime
- temp files are 4x normal
- reporting replica CPU is high

Context evidence:

- export worker is expected during weekly enterprise backfills
- exports can lag for two hours without customer impact
- checkout latency is also elevated, but machine severity is lower

Diagnosis implication:

- export workload may be noisy and worth throttling
- checkout path may deserve higher priority despite lower raw database severity
- diagnosis should separate primary customer-impacting path from secondary load

## Example: Moderate Machine Severity, High Business Priority

Machine finding:

- query family p95 increased moderately
- total runtime rank is only fifth
- errors are low

Context evidence:

- query belongs to checkout discount evaluation
- deploy shipped 15 minutes before latency spike
- large carts are high revenue and currently affected

Diagnosis implication:

- finding should move up in priority
- causal chain should focus on the checkout deploy and request fanout
- mitigation should be evaluated before lower-risk maintenance findings

## Output Artifact

The diagnosis artifact should include:

- prioritized diagnosis list
- machine evidence summary for each diagnosis
- context evidence summary for each diagnosis
- assumptions and unknowns
- causal chain
- confidence level
- falsification checks
- findings treated as secondary or noisy
- operator validation notes

## Agent Step Risk Analysis

Scores:

- `No risk`: bounded, checkable, structured, and verifiable enough for agent
  acceleration with normal human review.
- `Risk`: useful, but can bias the engagement if not reviewed or constrained.
- `Unknown`: not enough examples yet to decide whether the agent role is
  reliable.

| Agent step | Score | Why |
| --- | --- | --- |
| Map findings to services, jobs, tenants, endpoints, product workflows, or data pipelines | Unknown | The reliability depends on the customer's naming conventions, service catalog, and ownership docs. |
| Compare findings against deploy, migration, feature-flag, maintenance, and bulk-job timelines | Risk | Temporal correlation can be misleading; the agent must state assumptions and alternatives. |
| Identify likely secondary effects and known-benign noise | Risk | Suppressing the wrong signal can hide the real incident; exclusions must be reversible and human-reviewed. |
| Propose a priority order that weighs machine severity against context impact | Risk | This is the core fuzzy judgment loop and requires operator validation. |
| Identify assumptions, unknowns, and falsification checks | No risk | This makes the diagnosis easier to challenge and does not by itself choose the answer. |

Information that would improve the scores:

- historical diagnosis memos and ground truth
- examples where high machine severity was low business priority
- examples where moderate machine severity was high business priority
- operator review notes on weighting mistakes
- explicit confidence and falsification templates

## Working Thesis

Company-aware diagnosis is where agent and human judgment become central, but
only after deterministic findings and context evidence exist.

The agent should make the join and weighting legible. Humans should validate the
business and operational interpretation before the diagnosis becomes the basis
for recommendations.
