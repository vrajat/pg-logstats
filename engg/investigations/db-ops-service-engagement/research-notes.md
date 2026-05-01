# Research Notes

Status: living investigation notes  
Parent: [Database Operations Service Engagement](README.md)

## Authoring Guidance From Discussion

- Maintain this investigation as a running document set.
  - Rewrite or overwrite as the thinking changes.
  - Commit only when the user calls a checkpoint.
  - Keep checkpoint commits focused on the current discussion unit.
- Use a three-level structure.
  - `README.md`: executive summary and navigation.
  - Level 2 docs: one document per primary section.
  - Level 3 docs: deep dives for individual workflow steps or subtopics.
- Keep the README useful as an executive summary.
  - Prefer bullets and tables over long prose.
  - Full sentences are fine when they clarify the point.
  - Avoid making it so terse that the meaning is lost.
  - Use notes below tables when the table needs interpretation.
  - Bubble up the most important conclusions from deep dives.
- Do not cross-link deep dives to each other by default.
  - Bring ideas together in the higher-level summaries instead.
- Keep the writing direct and engineering-oriented.
  - Be succinct.
  - Avoid marketing language.
  - Avoid generic AI claims.
  - Use concrete workflow steps, artifacts, and boundaries.
- Do not overfit to `pg-logstats`.
  - Treat it as the first concrete trial and example.
  - Keep the broader target as database operations across Postgres, data
    integration, analytics, key-value stores, and similar production systems.
- Do not shoehorn agent workflows.
  - Clearly separate what should be scripts, OSS tooling, agents, and humans.
  - Use agents only where they reduce real workflow friction or improve
    context handling.
- Preserve the machine evidence vs context evidence distinction.
  - Machine evidence: logs, metrics, stats, pooler state, replica state, CDC,
    and deterministic findings.
  - Context evidence: ownership, apps, runbooks, deploys, docs, operator
    heuristics, business impact, and safety boundaries.
- Tag reviewed workflow sub-steps when helpful.
  - `[script]`: deterministic collection, validation, scoring, or
    transformation.
  - `[agent]`: guided workflow, tool-loop driving, summarization, gathering,
    contradiction detection, or targeted follow-up generation.
  - `[human]`: severity judgment, context validation, safety approval,
    weighting approval, or action decision.
- For deep dives, clarify:
  - what is deterministic
  - where agents help
  - where humans must approve or validate
  - what artifact the step produces
  - examples when the work is abstract
- Keep standardization tentative unless explicitly decided.
  - Use example shapes to crystallize the work.
  - Avoid prematurely locking a schema.

## Current Open Questions

- Should the first design partner trial focus on a historical incident or a
  current recurring degradation?
- Which incident class should be first: slow queries, replica lag, CDC
  correctness, storage growth, or cost?
- What is the smallest evidence package that can still support principal-level
  recommendations?
- Should the final service artifact be optimized for incident command,
  executive communication, or engineering follow-up?
- What information can be safely requested from customers without requiring
  privileged production access?

## External Inputs Reviewed

- pgBadger documentation: <https://access.crunchydata.com/documentation/pgbadger/latest/>
- pgBadger incremental reports announcement:
  <https://www.postgresql.org/about/news/pgbadger-5-analyze-your-logs-daily-with-the-incremental-mode-1505/>
- Community pgBadger usage recap:
  <https://techcommunity.microsoft.com/blog/adforpostgresql/community-insights-on-pgbadger-a-pgsql-phriday-010-recap/3880911>
- pgBadger issue on hourly RDS incremental reports:
  <https://github.com/darold/pgbadger/issues/697>
- GitLab incident: statement timeouts and query planner statistics:
  <https://gitlab.com/gitlab-com/gl-infra/production/-/issues/3875>
- GitLab incident review: replica and primary saturation from expensive query
  traffic:
  <https://gitlab.com/gitlab-com/gl-infra/production/-/issues/20692>
- PostgreSQL mailing list incident: large table, autovacuum, and explosive
  replication lag:
  <https://www.postgresql.org/message-id/CA%2BB3G4QvCy_a2%2BSUbbeQU5Q0TzUrH2iUYKg-XBezeJDMfVoq7Q%40mail.gmail.com>
- PostgreSQL monitoring documentation:
  <https://www.postgresql.org/docs/current/monitoring.html>
- PostgreSQL CDC failure modes:
  <https://streams.dbconvert.com/blog/postgresql-cdc-breaks-in-production/>
