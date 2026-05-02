# Evidence Required Before Diagnosis

Status: living deep dive  
Parent: [Successful engagement criteria, workflow, and timeline](../successful-engagement.md)

## Question

What evidence is required before the engagement can make useful
recommendations?

## Framing

Diagnosis needs two evidence types:

- machine evidence: logs, metrics, PostgreSQL statistics, pooler state, replica
  state, CDC/logical replication state, and outputs from utilities such as
  `pg-logstats`
- context evidence: ownership, deploy history, runbooks, operator heuristics,
  business impact, known noisy workloads, and safety boundaries

Machine evidence shows what changed. Context evidence explains what that change
means and what actions are safe.

## Minimum Bar

Before diagnosis, the engagement should know:

- the incident window and at least one useful baseline or why no baseline exists
- whether the provided logs, metrics, and statistics are complete enough to
  trust
- what redaction changed or preserved
- which service, owner, workflow, or customer impact the main findings may map
  to
- which missing evidence could change the recommendation

## Keep

- customer-side evidence generation where possible
- explicit missing-evidence notes
- source-linked facts with freshness and confidence
- operator correction of the context model

## Cut Or Be Skeptical Of

- diagnosis from logs alone
- broad evidence requests unrelated to the incident
- reusable customer context packs without freshness discipline
- suppression rules that cannot be audited or reversed

## Agent Risk

- Low risk: turn missing evidence into targeted follow-up questions.
- Risk: drafting an operational model can overstate stale or incomplete context.
- Unknown: reusable customer context packs are useful only if freshness and
  ownership are maintained.
- Not worth it: evidence gathering that creates more access friction than it
  removes.

## Working Thesis

Machine evidence can often be made self-serve. Context evidence is the harder
part and likely the service bottleneck. The engagement should begin only when
both are good enough to support a recommendation, or when the missing evidence
is explicitly accepted as a risk.
