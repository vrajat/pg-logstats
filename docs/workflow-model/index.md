# Workflow Model

`pg-logstats` is not a general SQL shell for agents.

It is a controlled triage loop built around three ideas:

- the product packages the runbook
- the agent provides judgement at explicit branch points
- the product remains the gateway for approved diagnostic SQL

## Canonical Investigation Loop

The intended loop is:

1. install `pg-logstats`
2. install the agent guidance
3. run `inspect`
4. confirm that the environment is ready
5. run one bounded triage workflow
6. review the ranked findings and `next_actions[]`
7. select one approved next action
8. stop or escalate explicitly

This is a bounded investigation path, not a free-form exploration model.

## Runbook And Judgement

The important split is:

- `pg-logstats` owns the runbook, evidence shape, and action graph
- the agent owns the judgement when several branches are plausible

That is why the workflow pages in this docs set focus on what is being
automated, not on teaching a human operator to run each command manually.

## SQL Control Path

Follow-up SQL is executed through the product action model, not through
arbitrary SQL text.

The agent should:

- read the report
- choose a valid `action_id`
- invoke `pg-logstats run-sql` with the audit linkage fields

The product should:

- validate that the action is still allowed
- bind the required parameters
- execute the built-in SQL
- record the investigation step

## Failure Model

For beta, the product should prefer an honest stop over a weak partial result.

That means:

- no historical ranking without log-backed evidence
- no temp-file triage without the required temp-file evidence
- no arbitrary SQL execution through the product path
- no pretending that degraded evidence is sufficient

## Read Next

- [Investigation Guidance](../user-guide/guidance.md)
- [Workflow References](../user-guide/index.md)
