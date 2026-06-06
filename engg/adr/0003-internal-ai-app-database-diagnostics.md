# ADR 0003: Internal AI App Database Diagnostics

- Status: Proposed
- Date: 2026-05-29

## Context

`pg-logstats` already has two accepted direction-setting decisions:

- [ADR 0001](0001-ripgrep-inspired-cli-philosophy.md): keep the product
  CLI-first, investigation-oriented, fast, and composable
- [ADR 0002](0002-select-v1-investigation-workflows.md): prioritize
  `top query-families`, `errors`, and `temp-files` as the initial workflows

What is still missing is a precise decision about the first operating context
those workflows are meant to serve.

The strongest near-term operating context is not broad PostgreSQL observability.
It is **routine diagnostics for low-to-medium criticality internal applications
that are built quickly with AI assistance and backed by PostgreSQL**.

That environment has a specific failure pattern:

- many internal applications share a database or database fleet
- those applications often have uneven query quality and weak instrumentation
- each incident is too small to justify deep SRE attention
- in aggregate, the incidents still create real database pressure
- agents or AI-assisted developers are likely to be in the loop before an SRE is
  involved

The common operational question is not "generate a broad report." It is:

> Which internal app is causing PostgreSQL pain right now, what evidence proves
> it, what SQL is safe to run next, and when should the investigation stop and
> escalate?

This use case imposes constraints on the product:

1. the output must be compact enough for an agent turn
2. the output must remain auditable by a human SRE
3. the tool must not require an embedded agent runtime
4. live follow-up actions must be risk-labeled
5. app attribution must be first-class, not incidental
6. raw logs should stay outside agent context unless explicitly requested

## Decision

`pg-logstats` will treat **internal AI-app PostgreSQL diagnostics** as the first
product wedge and will optimize its workflow, output contract, and safety model
for that job.

### 1. Primary Job To Be Done

The first job is:

> produce a compact diagnostic packet that lets an agent or developer triage a
> PostgreSQL issue caused by an internal application and either propose a small
> fix or escalate to an SRE with evidence.

This decision is narrower than "PostgreSQL observability" and narrower than
"agent tooling." The scope is first-pass diagnosis of app-caused PostgreSQL
issues.

### 2. Primary Users

The primary users, in order, are:

1. a coding agent attached to an internal application repository
2. an AI-assisted developer debugging that application
3. an SRE or DBA receiving an escalation packet

The SRE is explicitly **not** the default first user for this workflow. The
product goal is to reduce routine SRE involvement by improving the quality of
the first-pass diagnostic packet.

### 3. Stable Interface Contract

The stable contract remains the CLI plus deterministic JSON output.

That means:

- the CLI is the canonical interface
- a skill or playbook is the first agent packaging layer
- MCP is a later adapter, not the product identity
- no workflow should require a specific agent runtime to be useful

Every important workflow must therefore be expressible as shell commands such as:

```text
pg-logstats top query-families --since 30m --output-format json
pg-logstats errors --since 30m --output-format json
pg-logstats temp-files --since 30m --output-format json
pg-logstats suggest-sql --findings-file findings.json --rank 1
```

### 4. Agent Packaging Decision

The intended agent consumers are the major coding-agent environments in current
practical use:

- Codex
- Claude Code
- Gemini CLI or similar agent shells

For these environments, the product decision is:

1. the CLI and JSON schema remain the canonical behavior contract
2. `pg-logstats` should be able to install harness-specific skills or playbooks
   for the Postgres diagnostics workflow
3. agent-specific skill packaging should be thin wrappers around one shared
   workflow, not three diverging implementations

The workflow logic that belongs in the shared skill layer is:

- when to run live preflight
- when to use log-backed workflows
- when to fall back to non-log sources
- how to interpret `verdict`
- which SQL risk labels permit the next step
- when to stop and escalate

`pg-logstats` should therefore own:

- one agent-neutral diagnostic playbook in version control
- thin agent-specific adapters only where a tool requires special file layout or
  metadata
- a CLI install path that places those adapters in the correct user-level
  location for each supported harness

The installation model should not depend on a git repository checkout being the
active working context. Early installation should be local, explicit, and
harness-specific.

That means the first installation model is:

- keep the shared playbook source in this repo
- install the harness-specific wrapper into the user-level location expected by
  the agent tool
- let `pg-logstats` perform the installation itself

This keeps the workflow auditable, versioned with the CLI, and usable even when
agent ecosystems change naming or packaging conventions.

Agent readiness for this workflow therefore means:

- the harness-specific guidance has been installed
- the agent environment has access to that installed guidance
- the guidance teaches mode detection, degraded behavior, risk handling, and
  escalation behavior

### 5. Environment Readiness Contract

This use case depends on being honest about which evidence sources are actually
available.

There are two supported operating modes:

1. `log_backed`
2. `live_only`

`log_backed` is the preferred mode for this ADR. It supports historical-window
triage and the main workflows in ADR 0002.

Minimum readiness for `log_backed` mode is:

- PostgreSQL logs are available from a supported source such as local files,
  rotated files, or RDS or CloudWatch export
- the log format is one that `pg-logstats` supports
- statements and durations are emitted in a way the tool can correlate
- temp-file and error signals are enabled if those workflows are expected
- application attribution is available through `application_name` where possible

For practical operations, this usually means the environment should be prepared
with some combination of:

- a supported `log_line_prefix` or a structured format such as `csvlog` or
  `jsonlog`
- statement logging sufficient for query-family analysis
- duration logging sufficient for runtime ranking
- `log_temp_files` when temp-file triage matters
- app-side `application_name` configuration

If those requirements are not met, the product must not pretend historical
diagnosis is available. It should downgrade explicitly to `live_only`.

`live_only` mode is restricted to non-log sources such as:

- `pg_stat_activity`
- `pg_stat_statements` when installed
- other lightweight live catalog or stats views that do not require log access

In `live_only` mode:

- historical log-window claims are out of scope
- some non-log aggregate history may still be available
- no workflow should claim complete query-family runtime attribution from logs
- findings should be marked as live-state findings, not log findings
- the tool should prefer preflight, active-query, and bounded catalog-based
  investigation

This is a deliberate product boundary. Missing logging is not a reason to invent
weak evidence; it is a reason to reduce scope and say so explicitly.

### 6. Diagnostic Packet Contract

The unit of output for this use case is a **diagnostic packet**.

Every packet must contain:

- `schema_version`
- `workflow`
- `analysis_window`
- `source_summary`
- `verdict`
- `verdict_reasons`
- `findings[]`

When relevant, a packet should also contain:

- `allowed_actions`
- `blocked_actions`
- `caution_notes`
- `commands_executed`

The packet must be small enough for an agent turn and specific enough for a
human handoff.

### 7. Required Finding Shape

Each finding in `findings[]` must answer five questions:

1. what is suspicious?
2. why is it ranked?
3. which app, user, database, or query family is involved?
4. what evidence supports the claim?
5. what should be inspected next?

Therefore each finding must include, directly or by workflow-specific subtype:

- stable `finding_id`
- `kind`
- `rank`
- `title`
- `reason_codes[]`
- dimensions for `application_name`, `database`, and `user` when known
- workflow metrics
- evidence handles
- suggested next SQL with risk labels when applicable

For this product wedge, `application_name` is a first-class field. Missing app
attribution is itself diagnostic information and should be visible in findings.

### 8. Safety Verdict Contract

Agents must not infer database safety heuristically from raw output. The packet
must expose an explicit verdict.

The allowed verdict values are:

- `clear`
- `busy`
- `saturated`
- `unknown`

The intended meaning is:

- `clear`: low-impact diagnostic reads are acceptable
- `busy`: restrict to low-impact and bounded reads
- `saturated`: stop adding investigative load and escalate
- `unknown`: insufficient live-state evidence to classify

The packet must also expose action classes instead of vague prose:

- `system_catalog_reads`
- `bounded_activity_queries`
- `explain_without_analyze`
- `large_unbounded_selects`
- `explain_analyze`

`allowed_actions` and `blocked_actions` may be omitted only when `verdict` is
`unknown`.

### 9. V1 Workflow Set For This Use Case

The first workflow set for this operating context is:

1. `top query-families`
2. `errors`
3. `temp-files`
4. `suggest-sql`

`running-queries` is pulled forward as the first new workflow after the current
V1 work because this use case requires a live-state preflight before agents run
heavier SQL.

The intended sequence is:

1. run live preflight when database access exists
2. inspect a short recent log window
3. select one ranked finding
4. request risk-labeled follow-up SQL
5. either propose an app fix or escalate

### 10. Command-Surface Implications

This decision commits the project to the following command-surface direction:

- short bounded windows should be a normal path, not a niche path
- app, user, and database attribution must be prominent in output
- evidence lookup must not require dumping full raw logs into the default output
- suggested SQL must be labeled by execution risk
- escalation-ready output is a product requirement, not an afterthought

It does **not** commit the project to adding every PostgreSQL diagnostic
workflow. The command surface should stay small.

### 11. Boundary Clarifications

The main purpose of this ADR is to remove a small number of likely
misinterpretations that would otherwise distort implementation.

- `pg-logstats` remains a CLI and evidence producer. It does not become an
  embedded agent runtime.
- This ADR prefers `log_backed` diagnostics, but allows reduced `live_only`
  investigation when supported logs are not available.
- The repository should own the shared diagnostic playbook and any thin
  agent-specific wrappers required for Codex, Claude Code, or Gemini-style
  environments.
- `pg-logstats` may suggest risk-labeled SQL, but SQL execution, remediation,
  and write-side actions remain outside this ADR.

`readiness` should be the authoritative capability report. Other workflows only
need to surface enough mode metadata to prevent misuse.

## Concrete Output Requirements

The exact JSON schema can still evolve, but the minimum contract for this use
case is fixed enough to guide implementation.

### Packet Example

```json
{
  "schema_version": 1,
  "workflow": "top_query_families",
  "analysis_window": {
    "since": "2026-05-29T10:00:00Z",
    "until": "2026-05-29T10:30:00Z"
  },
  "source_summary": {
    "kind": "cloudwatch_rds",
    "entries_scanned": 18420
  },
  "verdict": "busy",
  "verdict_reasons": ["long_running_queries_present"],
  "allowed_actions": [
    "system_catalog_reads",
    "bounded_activity_queries",
    "explain_without_analyze"
  ],
  "blocked_actions": ["large_unbounded_selects", "explain_analyze"],
  "findings": [
    {
      "finding_id": "query_family:app=invoice-helper:sql=...",
      "kind": "slow_query_family",
      "rank": 1,
      "title": "One internal app dominates recent query runtime",
      "reason_codes": ["high_total_runtime", "high_execution_count"],
      "application_name": "invoice-helper",
      "database": "internal_tools",
      "user": "app_user",
      "metrics": {
        "execution_count": 184,
        "total_duration_ms": 91200,
        "p95_duration_ms": 840
      },
      "evidence": {
        "sample_event_refs": ["evt_101", "evt_144", "evt_201"]
      },
      "next_sql": [
        {
          "label": "Find active sessions for this app",
          "risk": "safe",
          "sql": "select pid, application_name, state, wait_event_type, wait_event, query from pg_stat_activity where application_name = 'invoice-helper';"
        }
      ]
    }
  ]
}
```

### Risk Labels

The initial risk label set is:

- `safe`
- `bounded`
- `expensive`
- `requires_human_approval`

The label semantics are:

- `safe`: shared-memory or lightweight catalog inspection
- `bounded`: reads are scoped by app, pid, queryid, or a hard limit
- `expensive`: the query may create noticeable load even though it is read-only
- `requires_human_approval`: valid SQL exists, but the tool should not
  recommend execution by default

## Consequences

### Positive

- The project now has a precise first product wedge rather than a generic
  "agents plus Postgres" story.
- The output contract becomes concrete enough to guide schema, tests, and CLI
  design.
- App attribution becomes a product requirement instead of a convenience.
- The escalation handoff path becomes part of the main design, not secondary
  documentation.
- The skill and future MCP layers have a clear source-of-truth contract.

### Negative

- The project takes on a more opinionated safety model that must be defended in
  tests and docs.
- Some tempting workflows remain deferred even if they are technically feasible.
- The design is now less suitable as a generic PostgreSQL log-analysis pitch.

### Implementation Consequences

This decision increases pressure on the implementation plan in the following
areas:

1. `application_name`, user, and database must be preserved end-to-end in the
   event model and finding schema
2. `findings[]` JSON needs a tighter versioned contract
3. `suggest-sql` must emit risk labels
4. a live-state preflight path must be designed for `running-queries`
5. evidence handles need stable identifiers suitable for handoff
6. configuration must be available from V1 for thresholds, agent install paths,
   and local `suggest-sql` external rule commands
7. tests must verify deterministic ranking and stable reason codes

## Follow-Up Decisions Still Needed

This record intentionally leaves a few concrete decisions open:

- exact thresholds for `clear`, `busy`, and `saturated`
- exact user-level install layout for the shared playbook and agent-specific
  wrappers
- whether `running-queries` should accept captured `pg_stat_activity` input in
  addition to direct database access
- the minimum environment checklist for calling a system `log_backed`
- whether escalation packets are a dedicated command or a documented composition
  pattern
- whether `application_name` absence should map to a stable synthetic grouping
  field
