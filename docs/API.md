# pg-loggrep API Documentation

## Overview

pg-loggrep is a Rust library for parsing and analyzing PostgreSQL log files. It provides modules for parsing different log formats, analyzing query patterns and performance metrics, and formatting results in various output formats.

## Modules

### Parsers (`parsers`)

The parsers module contains implementations for different PostgreSQL log formats.

#### StderrParser

```rust
use pg_loggrep::StderrParser;

let parser = StderrParser::new();
let entries = parser.parse_lines(&log_lines)?;
```

**Methods:**
- `new() -> Self`: Create a new stderr parser
- `parse_line(&self, line: &str) -> Result<LogEntry, String>`: Parse a single log line
- `parse_lines(&self, lines: &[String]) -> Result<Vec<LogEntry>, String>`: Parse multiple log lines

### Analytics (`analytics`)

The analytics module provides tools for analyzing parsed log data.

#### QueryAnalyzer

```rust
use pg_loggrep::QueryAnalyzer;

let analyzer = QueryAnalyzer::new();
let analysis = analyzer.analyze_queries(&entries);
```

**Methods:**
- `new() -> Self`: Create a new query analyzer
- `analyze_queries(&self, entries: &[LogEntry]) -> QueryAnalysis`: Analyze queries from log entries
- `find_slow_queries(&self, entries: &[LogEntry], threshold_ms: u64) -> Vec<LogEntry>`: Find slow queries above a threshold

#### TimingAnalyzer

```rust
use pg_loggrep::TimingAnalyzer;

let analyzer = TimingAnalyzer::new();
let analysis = analyzer.analyze_timing(&entries);
```

**Methods:**
- `new() -> Self`: Create a new timing analyzer
- `analyze_timing(&self, entries: &[LogEntry]) -> TimingAnalysis`: Analyze timing patterns
- `calculate_percentiles(&self, response_times: &[Duration], percentiles: &[f64]) -> HashMap<f64, Duration>`: Calculate response time percentiles

### Output (`output`)

The output module provides formatters for different output formats.

#### JsonFormatter

```rust
use pg_loggrep::JsonFormatter;

let formatter = JsonFormatter::new();
let json_output = formatter.format_query_analysis(&analysis)?;
```

**Methods:**
- `new() -> Self`: Create a new JSON formatter
- `format_query_analysis(&self, analysis: &QueryAnalysis) -> Result<String, String>`: Format query analysis as JSON
- `format_timing_analysis(&self, analysis: &TimingAnalysis) -> Result<String, String>`: Format timing analysis as JSON
- `format_log_entries(&self, entries: &[LogEntry]) -> Result<String, String>`: Format log entries as JSON

#### TextFormatter

```rust
use pg_loggrep::TextFormatter;

let formatter = TextFormatter::new();
let text_output = formatter.format_query_analysis(&analysis)?;
```

**Methods:**
- `new() -> Self`: Create a new text formatter
- `format_query_analysis(&self, analysis: &QueryAnalysis) -> Result<String, String>`: Format query analysis as text
- `format_timing_analysis(&self, analysis: &TimingAnalysis) -> Result<String, String>`: Format timing analysis as text
- `format_log_entries(&self, entries: &[LogEntry]) -> Result<String, String>`: Format log entries as text

## Data Structures

### LogEntry

```rust
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub message: String,
    pub details: HashMap<String, String>,
}
```

### QueryAnalysis

```rust
#[derive(Debug)]
pub struct QueryAnalysis {
    pub total_queries: usize,
    pub slow_queries: Vec<LogEntry>,
    pub frequent_queries: HashMap<String, usize>,
    pub query_types: HashMap<String, usize>,
}
```

### TimingAnalysis

```rust
#[derive(Debug)]
pub struct TimingAnalysis {
    pub average_response_time: Duration,
    pub p95_response_time: Duration,
    pub p99_response_time: Duration,
    pub hourly_patterns: HashMap<u32, Duration>,
    pub daily_patterns: HashMap<u32, Duration>,
}
```

## Error Handling

All parsing and formatting methods return `Result<T, String>` where errors are returned as descriptive string messages. In production code, you may want to use a more sophisticated error type.

## Examples

See the `examples/` directory for complete usage examples.
