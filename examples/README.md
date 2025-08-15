# pg-loggrep Examples

This directory contains practical examples, sample log files, and common usage patterns for pg-loggrep.

## Directory Structure

```
examples/
├── README.md           # This file
├── logs/               # Sample PostgreSQL log files
│   ├── sample_stderr.log      # Basic stderr format examples
│   ├── performance_logs.log   # Performance-focused log samples
│   ├── error_scenarios.log    # Error and warning examples
│   └── mixed_workload.log     # Realistic mixed workload
├── queries/            # Common analysis patterns
│   ├── jq_patterns.md         # jq query patterns for JSON output
│   └── analysis_recipes.md    # Common analysis workflows
└── integration/        # Integration examples
    ├── basic_usage.rs         # Basic library usage
    ├── custom_parser.rs       # Custom parser implementation
    └── batch_processing.sh    # Batch processing scripts
```

## Quick Start Examples

### Basic Analysis
```bash
# Analyze a single log file
pg-loggrep --log-dir examples/logs --output-format text

# Get quick summary only
pg-loggrep --log-dir examples/logs --quick

# JSON output for further processing
pg-loggrep --log-dir examples/logs --output-format json > analysis.json
```

### Performance Analysis
```bash
# Focus on slow queries
pg-loggrep --log-dir examples/logs --output-format json | jq '.slow_queries[]'

# Sample large files for faster analysis
pg-loggrep --log-dir /var/log/postgresql --sample-size 10000
```

### Error Investigation
```bash
# Analyze error patterns
pg-loggrep --log-dir examples/logs --output-format json | jq '.error_summary'

# Verbose output for debugging
pg-loggrep --log-dir examples/logs --verbose
```

## Sample Log Files

### `logs/sample_stderr.log`
Basic PostgreSQL stderr format with common log entries including queries, connections, and system messages.

### `logs/performance_logs.log`
Performance-focused log entries with slow queries, execution times, and resource usage patterns.

### `logs/error_scenarios.log`
Various error conditions including syntax errors, connection failures, and system warnings.

### `logs/mixed_workload.log`
Realistic mixed workload combining OLTP and analytical queries with varying performance characteristics.

## Usage Patterns

See the individual files in the `queries/` and `integration/` directories for detailed examples and patterns.

## Contributing Examples

When adding new examples:

1. **Log Files**: Use realistic but anonymized data
2. **Documentation**: Include clear descriptions and expected outputs
3. **Testing**: Ensure examples work with current pg-loggrep version
4. **Variety**: Cover different use cases and scenarios

## Getting Help

- Check the main [README.md](../README.md) for basic usage
- See [docs/architecture.md](../docs/architecture.md) for technical details
- Run `pg-loggrep --help` for current CLI options
