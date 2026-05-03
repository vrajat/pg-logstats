# pg-logstats Documentation

This directory contains documentation for the pg-logstats project.

## Documentation Files

### API.md

Complete API documentation for the pg-logstats library, including:
- Module overview
- Data structures
- Method documentation
- Usage examples
- Error handling

### rds-cloudwatch.md

Operational guide for using `pg-logstats` with Amazon RDS PostgreSQL logs
published to CloudWatch Logs, including:

- RDS log export prerequisites
- `--rds-instance` and `--cloudwatch-log-group` usage
- time windows, filtering, and page limits
- LLM-friendly JSON workflows
- local RDS log fallback

## Building Documentation

To build the documentation locally:

```bash
cargo doc --no-deps --open
```

This will generate HTML documentation and open it in your default browser. The generated docs include the latest public API for:
- `parsers` (e.g., `StderrParser`)
- `analytics` (e.g., `QueryAnalyzer`, `TimingAnalyzer`)
- `output` (e.g., `JsonFormatter`, `TextFormatter`)

## Contributing to Documentation

When adding new features to pg-logstats:

1. Update the relevant module documentation in the source code
2. Update this API.md file if new public APIs are added
3. Add fixture-backed usage examples to the root README
4. Update the main README.md if necessary
