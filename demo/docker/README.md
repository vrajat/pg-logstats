# Docker Demo

This directory contains an optional PostgreSQL environment for generating logs.
For a faster walkthrough without PostgreSQL, use the fixture demo in
`../README.md`.

## Start PostgreSQL And Workload

```bash
docker compose up -d postgres
docker compose run --rm workload
```

The compose file defines:

- `postgres`: PostgreSQL 17 with statement and duration logging enabled.
- `workload`: a small SQL workload generator.

## Analyze Generated Logs

Copy generated PostgreSQL logs from the Docker volume or container to a host
directory, then analyze that directory:

```bash
pg-logstats top query-families --log-dir /path/to/copied/logs
```

For JSON:

```bash
pg-logstats top query-families \
  --output-format json \
  --outfile findings.json \
  --log-dir /path/to/copied/logs
```

To compare two copied log windows:

```bash
pg-logstats slow-queries diff \
  --baseline /path/to/baseline/logs \
  --target /path/to/target/logs
```

To print follow-up SQL from saved findings:

```bash
pg-logstats suggest-sql --findings-file findings.json --rank 1
```

## Log Format Note

`pg-logstats` reads PostgreSQL stderr logs with a prefix similar to:

```text
%m [%p] %u@%d %a:
```

If Docker-generated logs do not parse, first compare their prefix with the
fixture format in `../logs/sample_stderr.log`.

Use `pg-logstats --help` for the authoritative CLI options.
