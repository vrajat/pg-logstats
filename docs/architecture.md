# pg-loggrep Architecture

This document provides a comprehensive overview of the pg-loggrep system architecture, module responsibilities, data flow, and extension points for future development.

## Table of Contents

- [System Overview](#system-overview)
- [Module Architecture](#module-architecture)
- [Data Flow](#data-flow)
- [Core Components](#core-components)
- [Extension Points](#extension-points)
- [Performance Considerations](#performance-considerations)
- [Error Handling Strategy](#error-handling-strategy)
- [Testing Architecture](#testing-architecture)

## System Overview

pg-loggrep is designed as a modular, extensible PostgreSQL log analysis tool built in Rust. The architecture follows a pipeline pattern where data flows through distinct stages: discovery → parsing → analysis → output.

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   File Discovery│───▶│   Log Parsing   │───▶│   Analytics     │───▶│   Output        │
│                 │    │                 │    │                 │    │                 │
│ • Directory scan│    │ • Format detect │    │ • Query classify│    │ • JSON format   │
│ • File filtering│    │ • Line parsing  │    │ • Performance   │    │ • Text format   │
│ • Validation    │    │ • Error recovery│    │ • Aggregation   │    │ • Progress      │
└─────────────────┘    └─────────────────┘    └─────────────────┘    └─────────────────┘
```

### Design Principles

1. **Modularity**: Each component has a single responsibility and clear interfaces
2. **Extensibility**: New parsers, analyzers, and output formats can be added easily
3. **Performance**: Memory-efficient processing with sampling for large files
4. **Reliability**: Comprehensive error handling and graceful degradation
5. **Testability**: Each module is independently testable with comprehensive coverage

## Module Architecture

### Core Library Structure

```
src/
├── lib.rs              # Public API and core types
├── main.rs             # CLI interface and orchestration
├── parsers/            # Log format parsing
│   ├── mod.rs          # Parser trait and registry
│   └── stderr.rs       # PostgreSQL stderr format parser
├── analytics/          # Data analysis and metrics
│   ├── mod.rs          # Analysis orchestration
│   ├── queries.rs      # Query classification and analysis
│   └── timing.rs       # Performance metrics calculation
└── output/             # Result formatting and display
    ├── mod.rs          # Output trait and registry
    ├── json.rs         # JSON output formatter
    └── text.rs         # Human-readable text formatter
```

### Module Responsibilities

#### `src/lib.rs` - Core Types and Public API
- **Purpose**: Defines core data structures and public API
- **Key Types**:
  - `LogEntry`: Represents a parsed log entry
  - `AnalysisResult`: Contains analysis results and metrics
  - `PgLoggrepError`: Unified error type for the entire system
  - `LogLevel`: PostgreSQL log levels (DEBUG, INFO, NOTICE, WARNING, ERROR, FATAL, PANIC)
- **Responsibilities**:
  - Public API surface
  - Core data structure definitions
  - Error type definitions
  - Module re-exports

#### `src/main.rs` - CLI Interface and Orchestration
- **Purpose**: Command-line interface and workflow orchestration
- **Key Components**:
  - `Arguments`: CLI argument parsing with clap
  - `main()`: Application entry point and workflow coordination
  - Progress indication and user feedback
- **Responsibilities**:
  - CLI argument parsing and validation
  - File discovery and validation
  - Workflow orchestration
  - Progress reporting
  - Error handling and user feedback

#### `src/parsers/` - Log Format Parsing
- **Purpose**: Parse various PostgreSQL log formats into structured data
- **Architecture**:
  ```rust
  pub trait LogParser {
      fn parse_line(&self, line: &str) -> Result<Option<LogEntry>, PgLoggrepError>;
      fn supports_format(&self, sample: &str) -> bool;
  }
  ```
- **Current Implementations**:
  - `StderrParser`: PostgreSQL stderr format parser
- **Responsibilities**:
  - Format detection and validation
  - Line-by-line parsing with error recovery
  - Timestamp parsing and normalization
  - Multi-line statement handling

#### `src/analytics/` - Data Analysis and Metrics
- **Purpose**: Analyze parsed log data and generate insights
- **Key Components**:
  - `queries.rs`: Query classification and normalization
  - `timing.rs`: Performance metrics and statistical analysis
- **Responsibilities**:
  - Query type classification (SELECT, INSERT, UPDATE, DELETE, DDL, OTHER)
  - Query normalization for pattern analysis
  - Performance metrics calculation (percentiles, averages)
  - Slow query detection and analysis
  - Frequency analysis and aggregation

#### `src/output/` - Result Formatting
- **Purpose**: Format analysis results for different output targets
- **Architecture**:
  ```rust
  pub trait OutputFormatter {
      fn format(&self, result: &AnalysisResult) -> Result<String, PgLoggrepError>;
      fn supports_quick_mode(&self) -> bool;
  }
  ```
- **Current Implementations**:
  - `JsonFormatter`: Machine-readable JSON output
  - `TextFormatter`: Human-readable text output
- **Responsibilities**:
  - Result serialization and formatting
  - Quick mode vs. detailed output
  - Progress indication during output

## Data Flow

### 1. Initialization Phase
```
CLI Arguments → Validation → Configuration
```
- Parse and validate command-line arguments
- Check log directory existence and permissions
- Initialize configuration and progress tracking

### 2. Discovery Phase
```
Log Directory → File Discovery → Validation → File List
```
- Scan specified directory for log files
- Filter files by extension and readability
- Validate file formats and warn about issues
- Generate prioritized file processing list

### 3. Parsing Phase
```
File List → Format Detection → Line Parsing → LogEntry Stream
```
- Detect log format for each file
- Parse lines with error recovery
- Handle multi-line statements and continuations
- Generate stream of structured `LogEntry` objects

### 4. Analysis Phase
```
LogEntry Stream → Classification → Aggregation → AnalysisResult
```
- Classify queries by type and complexity
- Calculate performance metrics and statistics
- Detect patterns and anomalies
- Generate comprehensive analysis results

### 5. Output Phase
```
AnalysisResult → Formatting → Display/File Output
```
- Format results according to specified output format
- Handle quick mode vs. detailed output
- Write to stdout or specified output file

## Core Components

### LogEntry Structure
```rust
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub message: String,
    pub query: Option<String>,
    pub duration: Option<f64>,
    pub connection_id: Option<String>,
    pub database: Option<String>,
    pub user: Option<String>,
}
```

### AnalysisResult Structure
```rust
pub struct AnalysisResult {
    pub total_entries: usize,
    pub query_types: HashMap<String, usize>,
    pub performance_metrics: PerformanceMetrics,
    pub slow_queries: Vec<SlowQuery>,
    pub error_summary: ErrorSummary,
    pub time_range: (DateTime<Utc>, DateTime<Utc>),
}
```

### Error Handling
```rust
#[derive(Error, Debug)]
pub enum PgLoggrepError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Configuration error: {0}")]
    Config(String),
}
```

## Extension Points

### Adding New Log Parsers

1. **Implement the LogParser trait**:
   ```rust
   pub struct CustomParser;

   impl LogParser for CustomParser {
       fn parse_line(&self, line: &str) -> Result<Option<LogEntry>, PgLoggrepError> {
           // Custom parsing logic
       }

       fn supports_format(&self, sample: &str) -> bool {
           // Format detection logic
       }
   }
   ```

2. **Register in parser module**:
   ```rust
   pub fn get_parser(format: &str) -> Box<dyn LogParser> {
       match format {
           "stderr" => Box::new(StderrParser::new()),
           "custom" => Box::new(CustomParser::new()),
           _ => Box::new(StderrParser::new()), // default
       }
   }
   ```

### Adding New Analytics

1. **Extend AnalysisResult**:
   ```rust
   pub struct AnalysisResult {
       // existing fields...
       pub custom_metrics: CustomMetrics,
   }
   ```

2. **Implement analysis functions**:
   ```rust
   pub fn analyze_custom_patterns(entries: &[LogEntry]) -> CustomMetrics {
       // Custom analysis logic
   }
   ```

### Adding New Output Formats

1. **Implement OutputFormatter trait**:
   ```rust
   pub struct CustomFormatter;

   impl OutputFormatter for CustomFormatter {
       fn format(&self, result: &AnalysisResult) -> Result<String, PgLoggrepError> {
           // Custom formatting logic
       }

       fn supports_quick_mode(&self) -> bool {
           true
       }
   }
   ```

2. **Register in output module**:
   ```rust
   pub fn get_formatter(format: &str) -> Box<dyn OutputFormatter> {
       match format {
           "json" => Box::new(JsonFormatter::new()),
           "text" => Box::new(TextFormatter::new()),
           "custom" => Box::new(CustomFormatter::new()),
           _ => Box::new(TextFormatter::new()), // default
       }
   }
   ```

### Future Phase Extensions

#### Phase 2: Advanced Analytics
- **Real-time monitoring**: Stream processing capabilities
- **Machine learning**: Anomaly detection and pattern recognition
- **Alerting**: Threshold-based notifications
- **Extension Point**: `src/analytics/realtime/` module

#### Phase 3: Multi-format Support
- **CSV logs**: Structured log parsing
- **JSON logs**: Native JSON log support
- **Syslog**: System log integration
- **Extension Point**: `src/parsers/` additional implementations

#### Phase 4: Advanced Output
- **Web dashboard**: HTML/CSS/JS output
- **Grafana integration**: Metrics export
- **Database export**: Direct database insertion
- **Extension Point**: `src/output/` additional formatters

## Performance Considerations

### Memory Management
- **Streaming processing**: Process files line-by-line to minimize memory usage
- **Sampling**: Use `--sample-size` for large files to limit memory consumption
- **Lazy evaluation**: Parse and analyze data on-demand

### Processing Optimization
- **Regex compilation**: Compile regex patterns once and reuse
- **String interning**: Reuse common strings to reduce allocations
- **Parallel processing**: Future enhancement for multi-file processing

### Scalability Limits
- **Single-threaded**: Current implementation processes files sequentially
- **Memory bounds**: Large files are handled through sampling
- **Disk I/O**: Performance limited by disk read speed

## Error Handling Strategy

### Error Categories
1. **Recoverable Errors**: Continue processing with warnings
   - Malformed log lines
   - Unparseable timestamps
   - Missing optional fields

2. **Fatal Errors**: Stop processing with clear error messages
   - File not found
   - Permission denied
   - Invalid arguments

3. **Validation Errors**: Prevent processing with helpful guidance
   - Invalid log directory
   - Unsupported file formats
   - Configuration conflicts

### Error Recovery
- **Line-level recovery**: Skip malformed lines with warnings
- **File-level recovery**: Continue with next file on parse errors
- **Graceful degradation**: Provide partial results when possible

## Testing Architecture

### Test Organization
```
tests/
├── integration_tests.rs    # End-to-end CLI testing
├── unit/                   # Unit tests by module
│   ├── parser_tests.rs     # Parser unit tests
│   ├── analytics_tests.rs  # Analytics unit tests
│   └── output_tests.rs     # Output formatter tests
├── test_data/              # Test data generation
│   └── mod.rs              # Utilities for creating test data
└── README.md               # Testing documentation
```

### Testing Strategy
1. **Unit Tests**: Test individual functions and modules in isolation
2. **Integration Tests**: Test complete workflows and CLI interface
3. **Property-based Tests**: Test invariants and edge cases
4. **Performance Tests**: Validate performance characteristics
5. **Regression Tests**: Prevent regressions in core functionality

### Test Data Management
- **Generated test data**: Programmatically create various log scenarios
- **Deterministic tests**: Use fixed seeds for reproducible results
- **Edge case coverage**: Test boundary conditions and error cases
- **Performance benchmarks**: Measure and validate performance metrics

---

This architecture supports the current Phase 1 implementation while providing clear extension points for future phases. The modular design ensures that new features can be added without disrupting existing functionality, and the comprehensive testing strategy maintains reliability as the system evolves.
