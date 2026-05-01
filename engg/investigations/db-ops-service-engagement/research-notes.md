# Research Notes

Status: living investigation notes  
Parent: [Database Operations Service Engagement](README.md)

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
