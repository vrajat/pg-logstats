# About pg-logstats

**pg-logstats is a PostgreSQL-native log investigation CLI.**

It should feel closer to `ripgrep` than to a traditional database report generator: simple to invoke, useful by default, fast enough to stay in the loop, and composable with the rest of the shell.

Primary inspiration:

- Andrew Gallant, "ripgrep is faster than {grep, ag, git grep, ucg, pt, sift}" — <https://burntsushi.net/ripgrep/>

## Core Thesis

`psql` is already a strong live-state tool.

`pgBadger` is already a strong broad report generator.

The open space is a tool that turns PostgreSQL log streams into **small, structured investigative outputs** that help a human or agent answer:

- what changed?
- what is suspicious?
- what should I inspect next?
- which SQL should I run now?

## Inspired by ripgrep

The most important lesson from ripgrep is not just “be fast.” It is the combination of:

- simplicity
- utility
- performance

That combination is what makes a CLI become a habit instead of a curiosity.

For `pg-logstats`, that means:

- smart defaults
- compact outputs
- explicit escape hatches
- performance as a product feature

## Simplicity

`pg-logstats` should be easy to explain in one sentence:

> Search and triage PostgreSQL logs quickly, then jump into follow-up SQL.

The CLI should not begin with dozens of report modes or a dashboard mindset.

It should begin with a small set of obvious workflows:

- top query families
- top errors
- temp files
- lock-related events
- evidence for a finding
- suggested next-step SQL

Simple does not mean minimal at all costs. It means the common path is obvious and the command structure reflects real investigation tasks.

## Utility

The tool must be useful before it is comprehensive.

That means:

- work on rotated and offline logs
- normalize noisy queries into query families
- correlate statements, durations, errors, and plans
- preserve evidence while emitting compact summaries
- bridge into `psql` and PostgreSQL system views

The output should make the next step clearer, not just generate a prettier artifact.

## Performance

Performance is not an optimization pass. It is part of the product.

If the tool is too slow, users and agents will fall back to ad hoc shell pipelines or give up on iterative exploration.

The performance bar should support:

- tight local iteration
- incremental updates
- cheap top-N retrieval
- large log directories
- token-efficient machine-readable output

The fast path matters more than the long feature list.

## Smart Defaults

Like ripgrep, `pg-logstats` should do the right thing in the common case without a long incantation.

Examples of good defaults:

- prefer structured log formats when available
- show ranked findings instead of dumping raw matching lines
- return compact output by default
- preserve stable identifiers for query families and findings
- hide obvious noise only when the rule is explicit, explainable, and reversible

## Explicit Escape Hatches

Smart defaults are only tolerable when users can override them.

`pg-logstats` should make it easy to:

- inspect raw evidence
- disable filters
- widen or narrow time windows
- include noisy maintenance jobs
- switch between human-readable and machine-readable output

The tool should be opinionated, not opaque.

## PostgreSQL Specificity

The strongest reason for this project to exist is PostgreSQL-specific semantics.

The tool should understand and expose concepts such as:

- `queryid`
- `application_name`
- `pg_stat_statements`
- `auto_explain`
- temp files
- lock waits
- autovacuum and analyze activity
- PgBouncer-adjacent workflows

This should not become a generic log search engine.

## Composability

`pg-logstats` should work well with:

- `psql`
- `jq`
- `fzf`
- shell scripts
- Makefiles
- LLM skills and agents

It should be natural to pipe its output into another tool or to ask it for the next SQL query to run.

## Structured Output First

Human-readable output matters, but the durable asset is a stable machine-readable representation of findings and evidence.

That is what enables:

- shell composition
- automated triage
- LLM-friendly workflows
- historical diffing
- future sinks like JSONL or Parquet

Pretty reports can sit on top later.

## Honest Anti-Pitch

Do not use `pg-logstats` when you need:

- a ubiquitous POSIX-standard tool
- a general-purpose log search engine
- a full HTML reporting suite on day one
- a replacement for `psql`

Those boundaries are healthy. They keep the product sharp.
