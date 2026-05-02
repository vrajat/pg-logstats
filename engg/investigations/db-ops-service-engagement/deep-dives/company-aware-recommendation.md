# Company-Aware Recommendation

Status: living deep dive
Parent: [Successful engagement criteria, workflow, and timeline](../successful-engagement.md)

## Question

How should the engagement turn machine findings plus company context into a
recommendation the customer can act on?

## Framing

The primary output is not a standalone diagnosis. It is a recommendation brief
that includes the diagnosis, confidence, evidence, tradeoffs, and action path.

The highest machine-severity finding is not always the right priority. A noisy
export job, a checkout path, a support dashboard, and a CDC consumer can have
different business meaning even when database symptoms look similar.

## Keep

- a visible join between machine findings, context evidence, and action path
- explicit assumptions, unknowns, and falsification checks
- separation between immediate mitigation, root-cause repair, diagnostic
  follow-up, actions to avoid, and structural follow-up
- action options tied to evidence, owner, reversibility, blast radius, and
  approval boundary
- human approval for priority, confidence, and production-impacting action

## Cut Or Be Skeptical Of

- single blended scores that hide judgment
- treating temporal correlation as causation
- generic best-practice recommendations
- action plans that ignore customer runbooks or approval boundaries
- agent-generated causal or action narratives that operators must unwind

## Example

An export query may dominate total runtime, but if exports can lag for two
hours and checkout latency is customer-facing, the recommendation should not
blindly follow raw database cost. It might throttle exports, avoid failover,
watch checkout recovery, and leave structural follow-up for workload isolation.

## Agent Risk

- Low risk: list assumptions, unknowns, falsification checks, and actions to
  avoid.
- Risk: priority weighting, noise suppression, and action classification can
  change the recommendation; humans must validate them.
- Unknown: mapping findings to services, tenants, or product workflows depends
  on local context quality.
- Not worth it: formatting a recommendation when the customer's runbook already
  encodes the decision path.

## Working Thesis

This is the main service output. Agents may make the reasoning and options
easier to inspect, but humans own the business interpretation and action path.
