# Recommendation And Decision Framing

Status: living deep dive  
Parent: [Successful engagement criteria, workflow, and timeline](../successful-engagement.md)

## Question

How should the engagement convert a validated diagnosis into useful action
guidance?

## Current Framing

Recommendation framing is similar to company-aware diagnosis, but the output is
different.

Diagnosis answers: what is most likely happening, why, and with what confidence?

Recommendation framing answers: what should the company do now, what should it
avoid, what should it defer, and what should it change permanently?

This is a decision loop. Scripts can provide candidate action templates and risk
facts. Agents can structure the tradeoffs. Humans must approve the final action
path, especially anything production-impacting.

## Inputs

Recommendation inputs:

- prioritized diagnosis memo
- causal chain
- machine and context evidence summary
- confidence and falsification notes
- safe-action and approval-boundary notes
- owner map
- business criticality map
- known mitigation runbooks
- operator validation notes

## What Should Stay Script-Owned

Scripts and deterministic rules can provide reusable action facts:

- known follow-up SQL
- known Postgres risk classes
- candidate mitigation templates
- known reversibility categories
- owner and service mappings from validated context
- affected query families, databases, users, replicas, and CDC consumers
- expected time-to-effect where it is formulaic or runbook-defined

Scripts can support the decision, but they should not choose the mitigation.

## Agent Workflow Opportunity

The agent should organize the decision space.

Useful agent work:

- separate immediate mitigation from root-cause repair
- propose candidate actions with evidence links
- classify actions by reversibility, blast radius, time-to-effect, and approval
  boundary
- identify actions to avoid and explain why
- compare options against business priorities and safety constraints
- map each action to likely owners
- turn the recommendation into an incident-channel brief
- generate follow-up tickets, runbook patches, and instrumentation suggestions
- call out assumptions that would change the recommendation

The agent should frame tradeoffs, not silently decide them.

## Human Review Boundary

Humans own the action decision.

Human approval is required for:

- throttling or disabling workloads
- changing pooler settings
- changing Postgres configuration
- running live SQL beyond approved read-only checks
- rolling back deploys
- changing feature flags
- failing over, restarting, or scaling infrastructure
- accepting degraded functionality
- communicating customer or executive impact

The human role is to approve the risk tradeoff and organizational coordination
path.

## Decision Loop

The recommendation loop should be explicit:

1. Start with the validated diagnosis and confidence level.
2. List candidate immediate mitigations.
3. List actions to avoid until more evidence exists.
4. Classify each option by reversibility, blast radius, time-to-effect, and
   owner.
5. Compare options against business priority and safety boundaries.
6. Ask targeted human questions where the answer changes the action path.
7. Produce an action recommendation with alternatives and deferrals.
8. Convert structural follow-up into tickets, runbook changes, or policy
   proposals.

## Recommendation Categories

The output should separate:

- immediate mitigation: what to do now to reduce customer or platform impact
- root-cause repair: what to fix after the incident is stabilized
- diagnostic follow-up: what evidence would change the recommendation
- actions to avoid: tempting moves that are not supported or are too risky
- structural follow-up: instrumentation, runbook, ownership, policy, or
  architecture changes

This prevents the team from mixing incident response with permanent repair.

## Example: Export Worker Versus Checkout

Validated diagnosis:

- export worker is creating heavy load and temp files
- checkout latency is the customer-impacting path
- export lag is acceptable for two hours
- checkout feature flag has a reversible degradation path

Recommendation framing:

- immediate mitigation: cap export-worker concurrency
- second mitigation: disable discount enrichment for large carts if checkout
  latency does not recover
- avoid: failover, because evidence points to workload shape, not primary
  instability
- follow-up: separate export workers from checkout pool capacity
- owner path: Data Platform for exports, Payments Platform for checkout,
  Incident Commander for customer-impacting degradation approval

## Example: Replica Lag And CDC

Validated diagnosis:

- WAL generation increased after a bulk job
- reporting replica lag is high
- CDC billing feed is lagging but within tolerance
- support dashboard staleness is customer-visible

Recommendation framing:

- immediate mitigation: throttle or pause the bulk job
- avoid: increasing read traffic on the lagging replica
- diagnostic follow-up: confirm slot lag trend after throttling
- structural follow-up: classify CDC consumers by business criticality and lag
  tolerance
- owner path: Data Engineering for bulk job, Support Systems for dashboard,
  Database team for replication monitoring

## Output Artifact

The recommendation artifact should include:

- recommended action path
- alternatives considered
- actions to avoid
- evidence links
- reversibility
- blast radius
- time-to-effect
- owner map
- approval boundaries
- assumptions and unknowns
- immediate mitigation checklist
- follow-up ticket list
- runbook, instrumentation, policy, or architecture recommendations

## Working Thesis

Recommendation framing is where the service becomes principal-level rather than
diagnostic-only.

The agent can make the decision space explicit and fast to review. Humans should
approve the actual tradeoff, especially when the recommendation affects
production behavior, customer experience, or cross-team ownership.
