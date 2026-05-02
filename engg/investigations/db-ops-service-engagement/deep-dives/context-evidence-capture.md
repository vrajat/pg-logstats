# Context Evidence Capture

Status: living deep dive  
Parent: [Successful engagement criteria, workflow, and timeline](../successful-engagement.md)

## Question

How should the engagement gather the company context needed to interpret
machine findings?

## Framing

Machine evidence says what changed. Context evidence explains what that change
means inside the company: who owns it, what recently changed, what is expected
noise, what hurts customers, and what mitigations are safe.

The goal is not to crawl every internal document. The goal is to gather enough
source-linked context to keep diagnosis from guessing.

## Keep

- source artifacts tied to specific machine findings
- operator commentary that corrects or confirms the draft context
- ownership, deploy, workload, business-impact, known-noise, and safety notes
- source, freshness, and validation status for important claims

## Cut Or Be Skeptical Of

- broad requests for docs, screenshots, exports, or auth
- silently treating stale runbooks or service catalogs as truth
- ownership mapping when local naming conventions are inconsistent
- context packs that take more operator correction time than direct questioning

## Example

If a finding points at `application_name = checkout-api`, useful context is not
"describe the architecture." Useful context is whether that service owns the
query, what deployed near the spike, whether the path is customer-facing, and
which mitigations are allowed.

## Agent Risk

- Low risk: structure approved operator commentary and draft source-linked
  context for correction.
- Risk: extracting facts from stale docs can create false confidence unless
  freshness and validation status are explicit.
- Unknown: ownership mapping depends on local naming quality and service
  catalog reliability.
- Not worth it: broad access requests that create security review without
  changing diagnosis.

## Working Thesis

Agents may help operators correct a draft model faster than starting from a
blank page. The value is source-linked context plus human validation, not
autonomous interpretation.
