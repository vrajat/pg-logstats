# Intake And Triage

Status: living deep dive  
Parent: [Successful engagement criteria, workflow, and timeline](../successful-engagement.md)

## Question

How should intake decide whether a production database operations case is worth
a fast professional service engagement?

## Framing

Intake is a readiness gate. It should answer whether the case is severe enough,
evidence-backed enough, and staffed enough to start the engagement clock.

An OSS tool such as `pg-logstats` can help by giving the customer a local,
shareable machine-evidence packet. That packet can make the conversation more
concrete, but it cannot decide whether the engagement is valuable.

## Keep

- deterministic checks for parseability, incident-window coverage, baselines,
  missing evidence, and redaction quality
- a short intake brief that summarizes the customer narrative, machine signals,
  contradictions, missing context, and proposed first branch
- a human decision on severity, service fit, operator availability, safety
  boundaries, and whether the case should proceed

## Cut Or Be Skeptical Of

- treating the first incident label as a meaningful agent task
- starting the clock before evidence and operator availability are clear
- accepting cases that are really broad health checks or requests for guaranteed
  root cause

## Example

A customer reports checkout latency. The evidence packet shows logs from an
analytics replica and no read-routing context. Intake should not proceed as a
checkout diagnosis. It should route the case to `Needs context evidence` until
the operator can explain which traffic uses that replica.

## Agent Risk

- Low risk: summarize notes, machine signals, contradictions, and missing
  evidence into a reviewable brief.
- Risk: suggesting the first investigation branch can bias the engagement path;
  it needs evidence and human approval.
- Not worth it: obvious incident labeling if a human can do it in minutes.

## Working Thesis

Good intake prevents the service from spending expert time on unready cases.
Scripts make readiness concrete; agents may organize the packet; humans decide
whether the engagement is valuable and safe.
