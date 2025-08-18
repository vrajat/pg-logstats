# pg-loggrep

A fast, modern PostgreSQL log analysis tool written in Rust. Analyze PostgreSQL logs with powerful query classification, performance metrics, and flexible output formats.

[![Rust](https://github.com/yourusername/pg-loggrep/workflows/Rust/badge.svg)](https://github.com/yourusername/pg-loggrep/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Crates.io](https://img.shields.io/crates/v/pg-loggrep.svg)](https://crates.io/crates/pg-loggrep)

## 🚀 Quick Start

```bash
# Install from source
git clone https://github.com/yourusername/pg-loggrep.git
cd pg-loggrep
cargo install --path .

# Analyze a single log file
pg-loggrep /var/log/postgresql/postgresql.log

# Analyze all logs in a directory with JSON output
pg-loggrep --log-dir /var/log/postgresql --output-format json

# Quick summary of large files
pg-loggrep --log-dir /var/log/postgresql --quick --sample-size 10000
```

## 📋 Table of Contents

- [Features](#features)
- [Installation](#installation)
- [Usage](#usage)
- [Examples](#examples)
- [Demo](#demo)
- [Architecture](#architecture)
- [Contributing](#contributing)
- [License](#license)

## ✨ Features

### Phase 1 (Current)
- **Fast PostgreSQL Log Parsing**: Supports stderr format with comprehensive error handling
- **Query Analysis**: Automatic classification (SELECT, INSERT, UPDATE, DELETE, DDL, etc.)
- **Performance Metrics**: Duration analysis with percentiles and slow query detection
- **Flexible Output**: Human-readable text or structured JSON output
- **Large File Support**: Memory-efficient processing with sampling options
- **Progress Indication**: Real-time progress bars and verbose logging

### Planned Features
- **Multiple Log Formats**: CSV, JSON, syslog support
- **Advanced Analytics**: Query pattern detection, anomaly identification
- **Interactive Dashboard**: Web-based visualization and exploration
- **Real-time Monitoring**: Live log analysis and alerting
- **Export Options**: Integration with monitoring systems

## 🔧 Installation

### From Source (Recommended)

```bash
# Clone the repository
git clone https://github.com/yourusername/pg-loggrep.git
cd pg-loggrep

# Build and install
cargo install --path .

# Verify installation
pg-loggrep --version
```

### Prerequisites

- **Rust 1.70+**: Install from [rustup.rs](https://rustup.rs/)
- **PostgreSQL logs**: Ensure `log_statement = 'all'` and `log_duration = on` in postgresql.conf

### System Requirements

- **Memory**: 512MB minimum, 2GB recommended for large files
- **Storage**: Minimal disk space required (processes logs in-place)
- **OS**: Linux, macOS, Windows (cross-platform)

## 📖 Usage

### Basic Commands

```bash
# Analyze a single log file
pg-loggrep postgresql.log

# Analyze multiple files
pg-loggrep file1.log file2.log file3.log

# Analyze all logs in a directory
pg-loggrep --log-dir /var/log/postgresql/

# Limit analysis to first 1000 lines of each file
pg-loggrep --sample-size 1000 large-file.log

# Get quick summary without detailed queries
pg-loggrep --quick postgresql.log

# Output as JSON for further processing
pg-loggrep --output-format json postgresql.log | jq '.summary'
```

### Advanced Usage

```bash
# Combine multiple options
pg-loggrep \
  --log-dir /var/log/postgresql/ \
  --output-format json \
  --sample-size 5000 \
  --quick

# Process with verbose logging
RUST_LOG=debug pg-loggrep --verbose postgresql.log

# Save output to file
pg-loggrep --output-format json postgresql.log > analysis.json
```

### Command Line Options

| Option | Description | Example |
|--------|-------------|---------|
| `--log-dir <DIR>` | Directory containing log files | `--log-dir /var/log/postgresql/` |
| `--output-format <FORMAT>` | Output format: text, json | `--output-format json` |
| `--quick` | Show only summary information | `--quick` |
| `--sample-size <N>` | Limit analysis to first N lines | `--sample-size 10000` |
| `--verbose` | Enable verbose logging | `--verbose` |
| `--help` | Show help information | `--help` |
| `--version` | Show version information | `--version` |

## 💡 Examples

### Example 1: Basic Analysis

```bash
$ pg-loggrep sample.log
Query Analysis Report
===================
Total Queries: 1,234
Total Duration: 45,678.90 ms
Average Duration: 37.02 ms
P95 Duration: 156.78 ms
P99 Duration: 892.34 ms
Error Count: 12
Connection Count: 45

Query Types:
    SELECT: 856
    INSERT: 234
    UPDATE: 89
    DELETE: 34
       DDL: 12
     OTHER: 9

Slowest Queries:
     #  Duration (ms)  Query
     1       2,345.67  SELECT * FROM large_table WHERE complex_condition...
     2       1,234.56  CREATE INDEX idx_performance ON users(email, created_at)
     3         892.34  UPDATE users SET last_login = NOW() WHERE active = true

Most Frequent Queries:
     #     Count  Query
     1       234  SELECT * FROM users WHERE active = ?
     2       156  INSERT INTO logs (level, message) VALUES (?, ?)
     3        89  SELECT COUNT(*) FROM orders WHERE status = ?
```

### Example 2: JSON Output for Processing

```bash
$ pg-loggrep --output-format json sample.log | jq '.summary'
{
  "total_queries": 1234,
  "total_duration_ms": 45678.9,
  "avg_duration_ms": 37.02,
  "error_count": 12,
  "connection_count": 45
}
```

### Example 3: Large File Processing

```bash
$ pg-loggrep --sample-size 10000 --quick large-production.log
Processing large-production.log...
[████████████████████████████████████████] 10000/10000 lines

Quick Summary:
- Processed: 10,000 lines (sample)
- Total Queries: 8,456
- Average Duration: 23.45 ms
- Error Rate: 0.8%
- Top Query Type: SELECT (67.8%)
```

## 🎬 Demo

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

## 🏗️ Architecture

pg-loggrep is built with a modular architecture:

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

## 🤝 Contributing

Here's how to get started:

### Development Setup

```bash
# Clone and setup
git clone https://github.com/yourusername/pg-loggrep.git
cd pg-loggrep

# Install development dependencies
cargo build

# Run tests
cargo test

# Run with sample data
cargo run -- examples/sample.log
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
- **Backwards Compatibility**: Maintain API stability where possible

### Areas for Contribution

- **New Log Formats**: CSV, JSON, syslog parsers
- **Advanced Analytics**: Pattern detection, anomaly identification
- **Performance**: Optimization for very large files
- **Documentation**: Examples, tutorials, API docs
- **Testing**: Edge cases, performance benchmarks

## 📚 Documentation

- **[Architecture Guide](docs/architecture.md)**: System design and module overview
- **[API Documentation](https://docs.rs/pg-loggrep)**: Generated API docs
- **[Examples](examples/)**: Sample code and usage patterns
- **[Demo Guide](demo/README.md)**: Step-by-step demo walkthrough
- **[Testing Guide](tests/README.md)**: Running and writing tests

## 🔍 Troubleshooting

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
- Enable `--quick` mode for faster summary analysis
- Use `--sample-size` to process subset of large files
- Check disk I/O performance

### Getting Help

- **Issues**: [GitHub Issues](https://github.com/yourusername/pg-loggrep/issues)
- **Discussions**: [GitHub Discussions](https://github.com/yourusername/pg-loggrep/discussions)
- **Documentation**: Check [docs/](docs/) directory

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
