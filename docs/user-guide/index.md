# Workflow References

These pages document the PostgreSQL triage runbooks that `pg-logstats`
automates for agents.

They are best read as audit and workflow references:

- what signal the workflow is using
- what evidence it requires
- how findings are ranked
- what follow-up actions the product may allow
- where the workflow should stop and escalate

If you are looking for installation and readiness, start with [Setup](../setup/index.md).
If you are looking for the control-path model, start with
[Workflow Model](../workflow-model/index.md).

## Current Workflow Set

- [Slow Query Triage](top-query-families.md)
- [Error Triage](errors.md)
- [Temp File Triage](temp-files.md)
- [Investigation Guidance](guidance.md)
