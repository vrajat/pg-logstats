# Design Partner Workflow

Status: living investigation
Parent: [Database Operations Service Engagement](README.md)

## Goal

Use design partners to test three claims:

- production Postgres db-ops has incidents where services beat tooling alone
- OSS scripts can make intake and machine evidence faster
- agents can save time without adding unsupported claims or correction burden

`pg-logstats` is the first OSS trial. It is not the full service boundary.

## Partner Fit

| Fit Area | Strong Signal | Primary Risk | De-Risking Move |
| --- | --- | --- | --- |
| Production scale | Recurring SLA, cost, uptime, replica, CDC, or query-performance pain | Toy workload produces false confidence | Require one or two real incidents where the internal team struggled |
| Evidence access | Sanitized logs, metrics, timelines, docs, and runbooks are available | Trial becomes an interview with no machine evidence | Confirm evidence availability before scheduling the workflow test |
| Operator access | Named operators can explain what happened and what decision was hard | Context stays guesswork | Require an operator walkthrough and validation of artifacts |
| Buying signal | Faster diagnosis or action framing would have mattered | Partner wants a dashboard or generic report | Ask whether a recommendation brief would have changed the incident response |

## Trial Steps

| Step | Purpose | Primary Risk | De-Risking Move | Pass Signal |
| --- | --- | --- | --- | --- |
| 1. Incident selection | Pick a historical incident before live pressure is introduced | Weak incident makes the thesis look better than it is | Choose an incident with a real internal struggle, delayed decision, or unresolved RCA | Partner can name what was hard: evidence, context, prioritization, ownership, or action |
| 2. Services need review | Test whether expert judgment was actually needed | Tooling was enough and the engagement adds ceremony | Compare what the team tried, where it got stuck, and which decision remained unclear | A recommendation brief would have materially helped during the incident |
| 3. OSS script trial | Test whether customer-run scripts speed up machine evidence | Setup, log format, redaction, or missing data costs more time than it saves | Run `pg-logstats` locally on incident and baseline windows; record setup time, parseability, completeness, and useful findings | OSS output makes intake more concrete without requiring raw-log sharing |
| 4. Agent acceleration trial | Test whether agents beat a scripts-plus-template control path | Agents create broad narrative, unsupported claims, or unsafe action framing | Score each agent task as `No risk`, `Risk`, or `Unknown`; record human corrections and time saved or added | Agents reduce operator or expert time without hiding assumptions |
| 5. Ground truth review | Compare artifacts against what actually happened | Trial rewards plausible writing instead of correct judgment | Review corrected artifacts with operators and mark unsupported claims, missed signals, and wrong priorities | Risk scores change based on evidence, not vibes |
| 6. Live or near-live trial | Test speed, trust, and data friction under pressure | Moving too early creates operational risk and low trust | Run only after historical trials produce useful artifacts, documented OSS steps, and explicit approval boundaries | Customer trusts the brief enough to use it in an incident channel or follow-up plan |

## Agent Step Review

For each staged workflow step, record the agent task, risk score, correction
burden, and whether the same result would have been faster with a template.

| Workflow Step | Primary Agent Risk | De-Risking Move |
| --- | --- | --- |
| Intake and triage | First-branch selection can bias the engagement | Human approves the intake state and investigation branch |
| Machine evidence and analysis | Collection guidance may be wrong for the provider or topology | Test recipes against partner environments before trusting guidance |
| Context evidence capture | Ownership, service, and product-path mapping may be wrong | Require source links plus operator validation before use |
| Company-aware recommendation | Priority weighting and action classification shape production decisions | Human validates priority, confidence, actions to avoid, and production-impacting changes |

## Evidence To Capture

| Claim | Evidence |
| --- | --- |
| Services need | Incident duration, teams involved, unresolved RCA questions, delayed decisions |
| OSS value | Setup time, run time, parseability, completeness, redaction quality, usefulness for intake |
| Agent value | Time saved or added, human corrections, unsupported claims caught, operator trust, artifact reuse |

## Stop Conditions

Re-scope or stop the trial when:

- no real incident is available
- logs or metrics for the relevant window are unavailable
- no operator can validate context
- the partner wants a generic dashboard or health report
- agents require more correction time than they save
- the expected outcome is autonomous production change
