# Machine Evidence And Analysis

Status: living deep dive
Parent: [Successful engagement criteria, workflow, and timeline](../successful-engagement.md)

## Question

How should the engagement turn production machine evidence into reviewable
findings without turning the work into a broad health check?

## Framing

This step should be script-first. The goal is to collect enough relevant
machine evidence, validate it, and reduce it into ranked findings that a human
can challenge.

`pg-logstats` can help when PostgreSQL logs are available, but it is only one
evidence source. The engagement should also care about metrics, statistics
views, poolers, replicas, CDC/logical replication, and topology where relevant.

## Keep

- customer-side collection where possible, so raw production data does not need
  to leave the customer environment
- explicit redaction, parseability, completeness, and baseline notes
- source-linked findings that show what changed and what evidence supports it
- human approval for data sharing, live read-only SQL, and whether the artifact
  is credible enough to join with context

## Cut Or Be Skeptical Of

- agent-owned tool choice, threshold tuning, or suppression rules
- broad health-check collection when the engagement is incident-focused
- findings that are not reproducible or cannot point back to source evidence

## Example

For a slow-query incident, `pg-logstats` might rank changed query families and
suggest follow-up SQL. That is useful only if the log window, baseline, redaction
policy, and source references are clear enough for a human to challenge.

## Agent Risk

- Low risk: explain missing signals and prepare a source-linked walkthrough of
  deterministic findings.
- Risk: provider-specific collection guidance can waste time or collect the
  wrong evidence if recipes are not tested.
- Not worth it: tool choice, thresholds, suppression, and baseline selection
  should become scripts or explicit human-reviewed presets.

## Working Thesis

Machine evidence and deterministic analysis are one engagement phase. The useful
boundary is not "collection versus analysis"; it is script-owned machine
evidence versus company context that needs human validation.
