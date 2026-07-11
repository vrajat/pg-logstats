---
title: pg-logstats Rust API Reference
description: Reference the Rust library surface for pg-logstats parsers, analytics, and formatters used in PostgreSQL log analysis workflows.
schema:
  "@context": "https://schema.org"
  "@type": "TechArticle"
  headline: "pg-logstats Rust API Reference"
  description: "Rust API reference for pg-logstats parsing, analytics, and output modules."
  url: "https://pg-logstats.vrajat.com/reference/api/"
---

# pg-logstats API Documentation

## Overview

The primary product surface is the `pg-logstats` CLI and its persisted triage
reports. The Rust crate also exposes parser, normalization, analytics, finding,
report, and formatter types for embedding PostgreSQL log analysis in other
tools.

This page is an orientation to the public Rust surface. Treat the source
and generated Rust documentation as authoritative for exact signatures.

## Modules

### Parsers (`parsers`)

The parsers module contains implementations for different PostgreSQL log formats.

#### TextLogParser

```rust
use pg_logstats::{TextLogParser, Result};

let parser = TextLogParser::new();
let entries = parser.parse_lines(&log_lines)?;
```

`TextLogParser::new()` auto-detects the supported default text prefix and the
Amazon RDS `%t:%r:%u@%d:[%p]:` prefix. Use
`TextLogParser::with_format(TextLogFormat::AwsRds)` to force RDS parsing.

Common methods:
- `new() -> Self`
- `with_format(format: TextLogFormat) -> Self`
- `parse_line(&mut self, line: &str) -> Result<Option<LogEntry>>` — returns `Ok(None)` for unparseable/continuation lines
- `parse_lines(&self, lines: &[String]) -> Result<Vec<LogEntry>>`
- `parse_timestamp(&self, timestamp_str: &str, timezone: &str) -> Result<DateTime<Utc>>`
- `extract_duration(&self, message: &str) -> Option<f64>`

### Analytics (`analytics`)

The analytics module provides tools for analyzing parsed log data.

#### QueryAnalyzer

```rust
use pg_logstats::{QueryAnalyzer, Result};

let analyzer = QueryAnalyzer::new();
let analysis = analyzer.analyze(&entries)?;
```

Common methods:
- `new() -> Self`
- `with_settings(slow_query_threshold: f64, max_slow_queries: usize, max_frequent_queries: usize) -> Self`
- `analyze(&self, entries: &[LogEntry]) -> Result<AnalysisResult>`
- `analyze_events(&self, events: &[NormalizedEvent]) -> Result<AnalysisResult>`
- `find_slow_queries(&self, entries: &[LogEntry], threshold_ms: f64) -> Result<Vec<LogEntry>>`
- `normalize_query(&self, sql: &str) -> String`
- `classify_query(&self, sql: &str) -> QueryType`

#### TimingAnalyzer

```rust
use pg_logstats::{TimingAnalyzer, Result};

let analyzer = TimingAnalyzer::new();
let analysis = analyzer.analyze_timing(&entries)?;
```

Common methods:
- `new() -> Self`
- `with_config(config: TimingAnalyzerConfig) -> Self`
- `with_bucket_size(time_bucket_size: u32) -> Self`
- `analyze_timing(&self, entries: &[LogEntry]) -> Result<TimingAnalysis>`
- `analyze_timing_events(&self, events: &[NormalizedEvent]) -> Result<TimingAnalysis>`
- `calculate_percentiles(&self, response_times: &[f64], percentiles: &[f64]) -> Result<Vec<(f64, f64)>>`

### Output (`output`)

The output module provides formatters for different output formats.

#### JsonFormatter

```rust
use pg_logstats::JsonFormatter;

let formatter = JsonFormatter::new();
let json_output = formatter.format(&analysis)?;
```

Common methods:
- `new() -> Self`
- `with_pretty(pretty: bool) -> Self`
- `with_metadata(tool_version: impl Into<String>, log_files_processed: Vec<String>, total_log_entries: usize) -> Self`
- `format(&self, analysis: &AnalysisResult) -> Result<String>`
- `format_with_timing(&self, analysis: &AnalysisResult, timing: &TimingAnalysis) -> Result<String>`
- `format_findings(&self, findings: &FindingSet) -> Result<String>`
- `format_triage_report<T: Serialize>(&self, report: &PgTriageReport<T>) -> Result<String>`

#### TextFormatter

```rust
use pg_logstats::TextFormatter;

let formatter = TextFormatter::new();
let text_output = formatter.format_query_analysis(&analysis)?;
```

Common methods:
- `new() -> Self`
- `with_color(enable: bool) -> Self`
- `format_query_analysis(&self, analysis: &AnalysisResult) -> Result<String>`
- `format_timing_analysis(&self, analysis: &TimingAnalysis) -> Result<String>`
- `format_findings(&self, findings: &FindingSet) -> Result<String>`
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
    pub queries: Option<Vec<Query>>,
    pub duration: Option<f64>,
}
```

`LogEntry::normalized_query()` joins parsed statement fragments into the
normalized representation used by query-family analytics.

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
use pg_logstats::{PgLogstatsError, Result};
```

Most public methods return `Result<T>` where errors are `PgLogstatsError` variants, including `Io`, `Parse`, `TimestampParse`, `Configuration`, `Analytics`, `Serialization`, and `Unexpected`.

## Examples

See the root README and setup guide for CLI usage examples. Library consumers
usually start by parsing logs with `TextLogParser`, normalizing with
`normalize_log_entries`, and then building findings or triage reports from those
events.
