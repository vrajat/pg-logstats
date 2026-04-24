# pg-logstats

A fast, modern PostgreSQL log investigation tool written in Rust. Analyze PostgreSQL logs with explicit workflow commands, correlated query families, and structured findings output.

[![CI](https://github.com/vrajat/pg-logstats/actions/workflows/ci.yml/badge.svg)](https://github.com/vrajat/pg-logstats/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)


## Motivation

This project is a platform to experiment with AI Coding Assistants.

### Articles
The [intro post](https://vrajat.com/posts/pg-logstats-intro/) covers my goals and experiments building pg-logstats using AI code assistants.


## Quick Start

```bash
# Install from source
git clone https://github.com/yourusername/pg-logstats.git
cd pg-logstats
cargo install --path .

# Rank the top slow query families in a checked-in test fixture
pg-logstats top query-families tests/fixtures/cli/sample_stderr.log

# Save findings JSON from the same fixture corpus
pg-logstats top query-families \
  --output-format json \
  --outfile findings.json \
  tests/fixtures/cli/sample_stderr.log

# Diff a target fixture window against a baseline fixture window
pg-logstats slow-queries diff \
  --output-format json \
  --baseline tests/fixtures/cli/diff_baseline.log \
  --target tests/fixtures/cli/diff_target.log

# Print suggested follow-up SQL for the top finding in saved findings JSON
pg-logstats suggest-sql --findings-file findings.json --rank 1
```

## Table of Contents

- [Features](#features)
- [Installation](#installation)
- [Usage](#usage)
- [Examples](#examples)
- [Demo](#demo)
- [Architecture](#architecture)
- [Contributing](#contributing)
- [License](#license)

## Features

### Current
- **Workflow-first CLI**: Explicit `top query-families` and `slow-queries diff` commands
- **Correlated Query Families**: Statement and duration pairing using stderr process-order correlation
- **Structured Findings**: Text and JSON output from the same machine-readable finding model
- **Baseline Versus Target Diffing**: Deterministic regression ranking with explainable reason codes
- **Suggested Follow-up SQL**: Print `pg_stat_statements` and `pg_stat_activity` queries for a surfaced finding
- **Large File Support**: Memory-efficient processing with sampling options
- **Progress Indication**: Real-time progress bars and verbose logging

### Planned Features
- **Multiple Log Formats**: CSV, JSON, syslog support
- **Advanced Analytics**: Query pattern detection, anomaly identification
- **Interactive Dashboard**: Web-based visualization and exploration
- **Real-time Monitoring**: Live log analysis and alerting
- **Export Options**: Integration with monitoring systems

## Installation

### From Source (Recommended)

```bash
# Clone the repository
git clone https://github.com/yourusername/pg-logstats.git
cd pg-logstats

# Build and install
cargo install --path .

# Verify installation
pg-logstats --version
```

### Prerequisites

- **Rust 1.70+**: Install from [rustup.rs](https://rustup.rs/)
- **PostgreSQL logs**: Ensure `log_statement = 'all'` and `log_duration = on` in postgresql.conf

### System Requirements

- **Memory**: 512MB minimum, 2GB recommended for large files
- **Storage**: Minimal disk space required (processes logs in-place)
- **OS**: Linux, macOS, Windows (cross-platform)

## Usage

### Basic Commands

```bash
# Rank slow query families in a single log file
pg-logstats top query-families postgresql.log

# Rank slow query families across multiple files
pg-logstats top query-families file1.log file2.log file3.log

# Analyze all logs in a directory
pg-logstats top query-families --log-dir /var/log/postgresql/

# Limit analysis to first 1000 lines of each file
pg-logstats top query-families --sample-size 1000 large-file.log

# Output as JSON for further processing
pg-logstats --output-format json top query-families postgresql.log | jq '.findings'
```

### Advanced Usage

```bash
# Combine multiple options
pg-logstats \
  --output-format json \
  top query-families \
  --log-dir /var/log/postgresql/ \
  --sample-size 5000

# Process with verbose logging
RUST_LOG=debug pg-logstats top query-families postgresql.log

# Save output to file
pg-logstats --output-format json top query-families postgresql.log > findings.json

# Compare a target window to a baseline window
pg-logstats slow-queries diff \
  --baseline ./fixtures/baseline.log \
  --target ./fixtures/target.log

# Print the suggested SQL for a chosen finding from saved findings JSON
pg-logstats suggest-sql --findings-file findings.json --rank 1
```

### Command Line Options

| Option | Description | Example |
|--------|-------------|---------|
| `--output-format <FORMAT>` | Output format: text, json | `--output-format json` |
| `top query-families --log-dir <DIR>` | Analyze all logs in a directory | `top query-families --log-dir /var/log/postgresql/` |
| `top query-families --sample-size <N>` | Limit analysis to first N lines per file | `top query-families --sample-size 10000 postgresql.log` |
| `slow-queries diff --baseline <PATH> --target <PATH>` | Compare two explicit log windows | `slow-queries diff --baseline base.log --target target.log` |
| `suggest-sql --findings-file <PATH> --rank <N>` | Print follow-up SQL for one finding | `suggest-sql --findings-file findings.json --rank 1` |
| `--help` | Show help information | `--help` |
| `--version` | Show version information | `--version` |

## Examples

### Example 1: Top Query Families

```bash
$ pg-logstats top query-families tests/fixtures/cli/sample_stderr.log
Findings
Schema Version: 1

#1 [query_family:queryid=|db=appdb|user=app|app=api|sql=SELECT * FROM users WHERE id = ?]
Query family with high total runtime
Reason: 2 executions contributed 44.000 ms total runtime; max execution was 24.000 ms
Score: 44.000  Confidence: High
Query Family: queryid=|db=appdb|user=app|app=api|sql=SELECT * FROM users WHERE id = ?
SQL: SELECT * FROM users WHERE id = ?
```

### Example 2: JSON Findings Output

```bash
$ pg-logstats top query-families \
    --output-format json \
    tests/fixtures/cli/sample_stderr.log | jq '.findings[0]'
{
  "kind": "query_family",
  "rank": 1,
  "reason_codes": ["high_total_duration", "high_max_duration", "correlated_duration"]
}
```

### Example 3: Baseline Versus Target Diff

```bash
$ pg-logstats slow-queries diff \
    --output-format json \
    --baseline tests/fixtures/cli/diff_baseline.log \
    --target tests/fixtures/cli/diff_target.log \
    --min-target-count 3

{
  "findings": [
    {
      "kind": "slow_query_regression",
      "reason_codes": ["meets_eligibility_thresholds", "p95_regressed"],
      "baseline": { "...": "..." },
      "target": { "...": "..." },
      "delta": { "...": "..." }
    }
  ]
}
```

### Example 4: Suggested SQL

```bash
$ pg-logstats suggest-sql --findings-file findings.json --rank 1
#1 [query_family:queryid=|db=appdb|user=app|app=api|sql=SELECT * FROM users WHERE id = ?]
Query family with high total runtime
select queryid, calls, total_exec_time, mean_exec_time, rows, query from pg_stat_statements where query ilike '%SELECT * FROM users WHERE id = ?%' order by total_exec_time desc limit 20;
select pid, usename, datname, application_name, state, wait_event_type, wait_event, query_start, query from pg_stat_activity where datname = 'appdb' and usename = 'app' and application_name = 'api' order by query_start desc nulls last limit 20;
```

## Demo

We provide a complete Docker-based demo environment:

```bash
# Start the demo environment
cd demo
./scripts/setup.sh

# Run sample workload
./scripts/run-workload.sh

# Analyze generated logs
./scripts/run-demo.sh

# Cleanup
./scripts/cleanup.sh
```

See [demo/README.md](demo/README.md) for detailed demo instructions.

## Architecture

pg-logstats is built with a modular architecture:

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   CLI Interface │───▶│  Log Parser     │───▶│  Analytics      │
│                 │    │                 │    │                 │
│ • Argument      │    │ • Stderr Format │    │ • Query Class   │
│   Parsing       │    │ • Multi-line    │    │ • Performance   │
│ • File Discovery│    │   Statements    │    │ • Frequency     │
│ • Progress      │    │ • Error Handling│    │ • Patterns      │
└─────────────────┘    └─────────────────┘    └─────────────────┘
                                                        │
┌─────────────────┐    ┌─────────────────┐             │
│  Output Format  │◀───│  Results        │◀────────────┘
│                 │    │                 │
│ • Text Reports  │    │ • Aggregated    │
│ • JSON Export   │    │   Statistics    │
│ • Colored       │    │ • Query Lists   │
│   Output        │    │ • Metrics       │
└─────────────────┘    └─────────────────┘
```

For detailed architecture documentation, see [docs/architecture.md](docs/architecture.md).

## Contributing

Here's how to get started:

### Development Setup

```bash
# Clone and setup
git clone https://github.com/yourusername/pg-logstats.git
cd pg-logstats

# Install development dependencies
cargo build

# Run tests
cargo test

# Run with sample data
cargo run -- top query-families examples/sample.log
```

### Code Style

- Follow Rust standard formatting: `cargo fmt`
- Ensure clippy passes: `cargo clippy`
- Add tests for new features
- Update documentation for API changes

### Submitting Changes

1. **Fork** the repository
2. **Create** a feature branch: `git checkout -b feature/amazing-feature`
3. **Commit** your changes: `git commit -m 'Add amazing feature'`
4. **Push** to the branch: `git push origin feature/amazing-feature`
5. **Open** a Pull Request

### Development Guidelines

- **Write Tests**: All new features should include comprehensive tests
- **Document Code**: Use rustdoc comments for public APIs
- **Performance**: Consider memory usage and processing speed
- **Error Handling**: Provide clear, actionable error messages
- **CLI Clarity**: Prefer explicit workflow commands over implicit modes

### Areas for Contribution

- **New Log Formats**: CSV, JSON, syslog parsers
- **Advanced Analytics**: Pattern detection, anomaly identification
- **Performance**: Optimization for very large files
- **Documentation**: Examples, tutorials, API docs
- **Testing**: Edge cases, performance benchmarks

## Documentation

- **[Architecture Guide](docs/architecture.md)**: System design and module overview
- **[API Documentation](https://docs.rs/pg-logstats)**: Generated API docs
- **[Examples](examples/)**: Sample code and usage patterns
- **[Demo Guide](demo/README.md)**: Step-by-step demo walkthrough
- **[Testing Guide](tests/README.md)**: Running and writing tests

## Troubleshooting

### Common Issues

**"No log entries found"**
- Ensure log format matches PostgreSQL stderr format
- Check that `log_statement = 'all'` in postgresql.conf
- Verify file permissions and paths

**"Out of memory" errors**
- Use `--sample-size` to limit processing
- Process files individually rather than entire directories
- Consider upgrading system memory for very large files

**Slow processing**
- Use `top query-families --sample-size <N>` to process a subset of large files
- Check disk I/O performance

### Getting Help

- **Issues**: [GitHub Issues](https://github.com/vrajat/pg-logstats/issues)
- **Discussions**: [GitHub Discussions](https://github.com/vrajat/pg-logstats/discussions)
- **Documentation**: Check [docs/](docs/) directory

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
