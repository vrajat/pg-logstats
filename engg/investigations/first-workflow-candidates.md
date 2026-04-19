# First Workflow Candidates

Status: investigation  
Date: 2026-04-19

## Question

If “investigate new slow queries” is the long-term flagship workflow, what neighboring workflows are simpler to build and easier to test first?

## Evaluation Criteria

- easy to explain
- deterministic fixtures
- low ambiguity in ranking
- useful CLI output
- reuses the same event model where possible

## Candidate Workflows

### 1. Top Slow Query Families in One Window

Description:

- ignore baseline comparisons
- rank slow query families in a single target window

Why it is simpler:

- no novelty logic
- no historical diffing
- lower ranking ambiguity
- directly exercises statement/duration pairing and query-family grouping

Testability:

- high

This is the best stepping stone if the product still wants to end up at “new slow queries”.

### 2. New Error Classes

Description:

- group errors by SQLSTATE, normalized error text, app, user, or database
- surface error classes that are new or sharply increased

Why it is simpler:

- error lines are easier to detect than slow-query families
- fewer correlation requirements
- ranking can be frequency-based
- evidence is easier to preserve

Testability:

- very high

This is likely the easiest first “real” triage workflow.

### 3. Top Temp-File Producers

Description:

- identify queries, users, or applications associated with temp-file activity

Why it is simpler:

- narrow event type
- strong operational value
- deterministic fixtures possible

Testability:

- medium-high

This is a good second workflow after error classes or single-window slow queries.

### 4. Lock-Wait Event Triage

Description:

- group lock wait events and surface the top repeated patterns

Why it is useful:

- highly actionable
- operationally important

Why it is harder:

- event parsing and interpretation can be more varied
- useful grouping may require more context

Testability:

- medium

### 5. Noise-Suppressed Top Queries

Description:

- show top query families after excluding known maintenance or background workloads

Why it matters:

- reflects real operational use of pgBadger

Why it is still useful:

- teaches the CLI how to express investigation filters

Testability:

- medium-high

## Recommendation

If the product narrative must remain “investigate new slow queries,” then the safest implementation path is:

1. top slow query families in one window
2. two-window slow-query diff
3. new slow query families

If the product is willing to start with a simpler but still valuable user story, then:

1. new error classes

is probably the easiest first workflow to build and test.
