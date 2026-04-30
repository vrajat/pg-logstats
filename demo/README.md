# pg-logstats Demo

This directory contains a guided walkthrough for showing `pg-logstats` end to
end. The main path uses checked-in fixtures, so it does not require a running
PostgreSQL server.

Run from the repository root.

## Fixture Demo

Build or install the CLI:

```bash
cargo build
```

Rank query families:

```bash
cargo run -- top query-families demo/logs/sample_stderr.log
```

Save JSON findings:

```bash
cargo run -- top query-families \
  --output-format json \
  --outfile demo/findings.json \
  demo/logs/sample_stderr.log
```

Compare a target window against a baseline:

```bash
cargo run -- slow-queries diff \
  --baseline demo/logs/diff_baseline.log \
  --target demo/logs/diff_target.log
```

Print follow-up SQL:

```bash
cargo run -- suggest-sql --findings-file demo/findings.json --rank 1
```

Remove generated demo output when finished:

```bash
rm -f demo/findings.json
```

## Optional Docker Demo

`demo/docker/` contains a PostgreSQL environment that can generate fresh logs.

Use it when you want to inspect generated PostgreSQL logs:

```bash
cd demo/docker
docker compose up -d postgres
docker compose run --rm workload
```

Copy the generated logs to a host directory, then analyze compatible stderr logs
with the same workflow commands:

```bash
pg-logstats top query-families --log-dir /path/to/copied/logs
```

## Current CLI Surface

Supported commands:

- `pg-logstats top query-families`
- `pg-logstats slow-queries diff`
- `pg-logstats suggest-sql`

Supported output formats:

- `text`
- `json`
