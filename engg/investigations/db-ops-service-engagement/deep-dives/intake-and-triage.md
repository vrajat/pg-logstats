# Intake And Triage

Status: living deep dive  
Parent: [Successful engagement criteria, workflow, and timeline](../successful-engagement.md)

## Question

How should intake decide whether a production database operations case is a good
fit for a few-hour professional service engagement?

## Current Framing

Intake is a readiness gate, not just a sales qualification step.

The goal is to decide whether the case has enough severity, evidence, context
access, and operator availability to justify starting the engagement clock. An
OSS evidence tool like `pg-logstats` can shape the case, but it should not make
the final engagement decision.

## Role Of OSS Evidence Tooling

An OSS tool can help intake by producing a quick machine-evidence packet that
the customer can generate locally.

Useful outputs:

- parseability check for logs or exports
- incident-window coverage check
- baseline-window coverage check
- top changed query families
- top errors, waits, temp files, replication warnings, or pool signals
- evidence completeness score
- missing-evidence report
- redaction report
- suggested next data to collect
- stable JSON findings that can be shared without raw logs

This is machine evidence and scripting. It should be deterministic,
explainable, and runnable without giving external parties production access.

## What OSS Tooling Cannot Decide

The tool output is not enough to decide whether the engagement is valuable.

Human judgment is still required for:

- whether the incident is important enough
- whether the customer already tried the obvious fixes
- whether there is customer, revenue, or executive pressure
- whether the pain is recurring or one-off
- whether the right expert or operator is available
- whether recommendations would be actionable in the customer's organization
- whether the real issue is probably outside the database
- whether the customer wants diagnosis, validation, a second opinion, or
  political cover

These are context and service-fit questions. They can be structured, but they
should not be fully automated.

## Agent Workflow Opportunity

The agent should reduce expert time in intake, not replace expert judgment.

Useful agent work:

- review the OSS evidence packet
- turn messy customer notes into a structured intake brief
- generate targeted context questions
- identify contradictions between machine evidence and customer narrative
- distinguish missing machine evidence from missing context evidence
- suggest the first investigation branch
- prepare a reviewer packet for a human expert

Example contradictions:

- the incident is described as checkout latency, but the logs are from an
  analytics replica
- the customer says there was no deploy, but the timeline includes a migration
- the customer claims high QPS, but provided logs only include sampled slow
  statements
- the evidence packet shows replica lag, but no read-routing context is present

## Human Touch Required

A human reviewer should own the final intake decision.

The reviewer should decide:

- accept or reject the case
- whether the clock can start
- whether more machine evidence is required first
- whether more context evidence is required first
- whether the case should be re-scoped away from database operations
- whether the customer's expectations are aligned with what the service can
  responsibly deliver

The human touch matters because intake includes severity, trust, actionability,
politics, and safety boundaries.

## Intake Decision Matrix

| Intake question | Evidence type | Best mechanism |
| --- | --- | --- |
| Are logs parseable? | machine | OSS tool |
| Is the incident window covered? | machine | OSS tool |
| Is there a baseline? | machine | OSS tool |
| What changed in database signals? | machine | OSS tool plus scripts |
| What data is missing? | machine and context | completeness report plus agent |
| Is this likely database-centered? | mixed | agent draft, human review |
| Is this high-value enough? | context | human |
| Is the operator available? | context | human |
| Are recommendations likely actionable? | context | human |
| What should the first branch be? | mixed | agent recommendation, human approval |

## Intake Outcomes

Every intake should end in one of four states:

- `Ready`: enough machine evidence and context access exist to start the
  engagement clock.
- `Needs machine evidence`: the customer should run the collector, provide logs,
  add a baseline, or export stats before the engagement starts.
- `Needs context evidence`: the customer should provide ownership, deploy
  history, runbooks, operator walkthroughs, or business-impact context before
  the engagement starts.
- `Reject / re-scope`: the case is not production-impacting, too generic,
  missing access, outside the database operations boundary, or based on
  unrealistic expectations.

## Intake Artifact

The intake artifact should be short and reviewable.

It should include:

- case summary
- severity and business impact
- machine-evidence readiness
- context-evidence readiness
- likely incident class
- first recommended investigation branch
- missing evidence
- safety boundaries
- operator availability
- final intake state
- rationale for acceptance, rejection, or re-scope

## Working Thesis

OSS tooling can make intake faster and more concrete by producing local,
deterministic machine evidence. An agent workflow can organize the packet,
surface contradictions, and generate better questions.

The final intake decision should remain human-owned because the hard decision is
not only "is there a database signal?" but "is this a valuable, safe, and
actionable engagement?"
