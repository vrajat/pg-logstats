# Deterministic Analysis Of Machine Evidence

Status: living deep dive  
Parent: [Successful engagement criteria, workflow, and timeline](../successful-engagement.md)

## Question

How should the engagement turn collected machine evidence into useful findings,
and what role should agents and humans play?

## Current Framing

Deterministic analysis should be script-first.

The scripts and utilities produce the findings. The agent can drive the analysis
loop, run tools like `pg-logstats`, choose reasonable variations, and prepare a
walkthrough. Humans review the resulting artifacts before they are treated as
inputs to context capture and diagnosis.

## What Should Be Deterministic

The core findings should come from repeatable analysis:

- rank query families by total time, count, max, p95, and baseline delta
- identify errors, lock waits, temp files, autovacuum signals, checkpoint
  signals, replication warnings, CDC lag, and pool saturation
- compare incident, previous-window, same-hour-yesterday, and same-day-last-week
  windows where available
- correlate findings with database, user, `application_name`, pid, client,
  replica, and time window
- generate stable finding IDs
- preserve source references into logs, metrics, or snapshots
- emit machine-readable output
- generate suggested follow-up SQL and system-view checks

This work should be deterministic because the findings need to be auditable and
rerunnable.

## Agent Workflow Opportunity

The agent should be the driver of the deterministic tool loop, not the source of
truth.

Useful agent work:

- choose which deterministic tools to run based on the evidence bundle
- run `pg-logstats` and related scripts with the right windows and thresholds
- retry with adjusted thresholds when the first pass is too noisy or empty
- compare outputs across baseline windows
- build a concise walkthrough of the findings
- identify suspicious gaps that require more machine evidence
- prepare follow-up commands or SQL for human approval
- explain why a finding is ranked highly
- separate findings from hypotheses

The agent should not invent findings that are not supported by deterministic
outputs.

## Human Review Boundary

Humans do not need to watch every script run.

Human review is useful at the end of the analysis pass:

- confirm the artifact is credible
- reject findings that are obvious collection artifacts
- approve any live read-only follow-up SQL
- decide whether missing evidence blocks diagnosis
- decide whether the findings are ready to join with context evidence

The human role is artifact review and safety approval, not manual log analysis.

## Analysis Walkthrough

The output should be easy to review with an operator.

A good walkthrough should show:

- what changed most
- what is noisy but likely benign
- what is missing
- which findings have strong evidence
- which findings depend on weak or partial evidence
- which follow-up checks are suggested
- which context questions should be routed to the next step

The walkthrough should avoid causal claims until context evidence is joined.

## Output Artifact

The deterministic analysis artifact should include:

- ranked evidence table
- finding IDs
- source references
- analysis windows
- baseline windows used
- thresholds and filters used
- confidence in the machine evidence
- missing or low-quality inputs
- suggested follow-up SQL
- context questions generated from machine findings

## Working Thesis

This step benefits from agents as operators of deterministic tools. The agent
can run the loop faster than a human, keep the process organized, and prepare a
reviewable artifact.

The insight remains bounded: deterministic analysis says what changed and what
is suspicious in the machine evidence. It does not decide business importance,
ownership, safety, or root cause without context evidence and human review.
