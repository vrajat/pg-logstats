# pg-loggrep

PostgreSQL log parsing and analysis toolkit (library + CLI).

### Features

- PostgreSQL 17 stderr log parser (`StderrParser`)
  - Supports standard `log_line_prefix = '%m [%p] %q%u@%d %a: '`
  - Handles multi-line statements (continuation lines)
  - Normalizes SQL parameters (e.g., `$1`, `$2` → `?`)
  - Parses duration lines (emitted as separate entries)
  - Robust error handling via `thiserror`-based `PgLoggrepError`
- Analytics
  - `QueryAnalyzer`: totals, slowest/most-frequent queries, percentiles
  - `TimingAnalyzer`: average, p95/p99, hourly/daily patterns
- Output
  - `JsonFormatter` and `TextFormatter` for reports and raw entries
- Usable as both a library and a command-line tool

### Installation

Clone and build locally:

```bash
git clone https://github.com/<your-org-or-user>/pg-loggrep
cd pg-loggrep
cargo build
```

### CLI Usage

Run the binary (see `--help` for all options):

```bash
cargo run -- --help
```

Example (parse logs and print a text report):

```bash
cargo run -- --logfile "./logs/*.log" --format text
```

Example (JSON output):

```bash
cargo run -- --logfile "./logs/*.log" --format json
```

Note: The CLI options may evolve. Use `--help` for the authoritative reference.

### Library Usage

End-to-end example using the library API:

```rust
use pg_loggrep::{StderrParser, QueryAnalyzer, TimingAnalyzer, JsonFormatter, Result};

fn main() -> Result<()> {
    // 1. Collect log lines (e.g., from files or stdin)
    let log_lines: Vec<String> = vec![
        "2024-08-14 10:30:15.123 UTC [12345] postgres@testdb psql: LOG:  statement: SELECT 1".into(),
        "2024-08-14 10:30:15.456 UTC [12345] postgres@testdb psql: LOG:  duration: 0.123 ms".into(),
    ];

    // 2. Parse
    let parser = StderrParser::new();
    let entries = parser.parse_lines(&log_lines)?;

    // 3. Analyze
    let query_analysis = QueryAnalyzer::new().analyze_queries(&entries)?;
    let timing_analysis = TimingAnalyzer::new().analyze_timing(&entries)?;

    // 4. Format
    let json = JsonFormatter::new().format_query_analysis(&query_analysis)?;
    println!("{}", json);
    Ok(())
}
```

### Supported Log Formats

- PostgreSQL 17 stderr logs with `log_line_prefix = '%m [%p] %q%u@%d %a: '`

Notes:
- Statement lines are emitted as `LogLevel::Statement` entries with normalized `query`.
- Duration lines are emitted as `LogLevel::Duration` entries. They are not merged into the
  preceding statement entry in the current implementation.

### Development

Run tests:

```bash
cargo test
```

Build docs:

```bash
cargo doc --no-deps --open
```

### License

See `LICENSE` for details.
