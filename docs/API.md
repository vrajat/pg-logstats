# pg-loggrep API Documentation

## Overview

pg-loggrep is a Rust library for parsing and analyzing PostgreSQL log files. It provides modules for parsing different log formats, analyzing query patterns and performance metrics, and formatting results in various output formats.

## Modules

### Parsers (`parsers`)

The parsers module contains implementations for different PostgreSQL log formats.

#### StderrParser

```rust
use pg_loggrep::{StderrParser, Result};

let parser = StderrParser::new();
let entries = parser.parse_lines(&log_lines)?;
```

**Methods:**
- `new() -> Self`
- `parse_line(&mut self, line: &str) -> Result<Option<LogEntry>>` — returns `Ok(None)` for unparseable/continuation lines
- `parse_lines(&self, lines: &[String]) -> Result<Vec<LogEntry>>`

### Analytics (`analytics`)

The analytics module provides tools for analyzing parsed log data.

#### QueryAnalyzer

```rust
use pg_loggrep::{QueryAnalyzer, Result};

let analyzer = QueryAnalyzer::new();
let analysis = analyzer.analyze_queries(&entries)?;
```

**Methods:**
- `new() -> Self`
- `analyze_queries(&self, entries: &[LogEntry]) -> Result<AnalysisResult>`
- `find_slow_queries(&self, entries: &[LogEntry], threshold_ms: f64) -> Result<Vec<LogEntry>>`

#### TimingAnalyzer

```rust
use pg_loggrep::{TimingAnalyzer, Result};

let analyzer = TimingAnalyzer::new();
let analysis = analyzer.analyze_timing(&entries)?;
```

**Methods:**
- `new() -> Self`
- `with_bucket_size(time_bucket_size: u32) -> Self`
- `analyze_timing(&self, entries: &[LogEntry]) -> Result<TimingAnalysis>`
- `calculate_percentiles(&self, response_times: &[f64], percentiles: &[f64]) -> Result<Vec<(f64, f64)>>`

### Output (`output`)

The output module provides formatters for different output formats.

#### JsonFormatter

```rust
use pg_loggrep::JsonFormatter;

let formatter = JsonFormatter::new();
let json_output = formatter.format_query_analysis(&analysis)?;
```

**Methods:**
- `new() -> Self`
- `format_query_analysis(&self, analysis: &AnalysisResult) -> Result<String>`
- `format_timing_analysis(&self, analysis: &TimingAnalysis) -> Result<String>`
- `format_log_entries(&self, entries: &[LogEntry]) -> Result<String>`

#### TextFormatter

```rust
use pg_loggrep::TextFormatter;

let formatter = TextFormatter::new();
let text_output = formatter.format_query_analysis(&analysis)?;
```

**Methods:**
- `new() -> Self`
- `format_query_analysis(&self, analysis: &AnalysisResult) -> Result<String>`
- `format_timing_analysis(&self, analysis: &TimingAnalysis) -> Result<String>`
- `format_log_entries(&self, entries: &[LogEntry]) -> Result<String>`

## Data Structures

### LogEntry

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub process_id: String,
    pub user: Option<String>,
    pub database: Option<String>,
    pub client_host: Option<String>,
    pub application_name: Option<String>,
    pub message_type: LogLevel,
    pub message: String,
    pub query: Option<String>,
    pub duration: Option<f64>,
}
```

### AnalysisResult

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub total_queries: u64,
    pub total_duration: f64,
    pub query_types: HashMap<String, u64>,
    pub slowest_queries: Vec<(String, f64)>,
    pub most_frequent_queries: Vec<(String, u64)>,
    pub error_count: u64,
    pub connection_count: u64,
    pub average_duration: f64,
    pub p95_duration: f64,
    pub p99_duration: f64,
}
```

### TimingAnalysis

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimingAnalysis {
    pub average_response_time: Duration,
    pub p95_response_time: Duration,
    pub p99_response_time: Duration,
    pub hourly_patterns: HashMap<u32, f64>,
    pub daily_patterns: HashMap<u32, f64>,
}
```

## Error Handling

The library uses a unified error type:

```rust
use pg_loggrep::{PgLoggrepError, Result};
```

Most public methods return `Result<T>` where errors are `PgLoggrepError` variants, including `Io`, `Parse`, `TimestampParse`, `Configuration`, `Analytics`, `Serialization`, and `Unexpected`.

## Examples

See the `examples/` directory for complete usage examples.
