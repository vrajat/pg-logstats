# pg-loggrep Examples

This directory contains usage examples for the pg-loggrep library.

## Examples

### basic_usage.rs

Demonstrates how to:
- Parse PostgreSQL stderr logs using `StderrParser`
- Analyze queries with `QueryAnalyzer` and timing with `TimingAnalyzer`
- Format results using `JsonFormatter` and `TextFormatter`

Run the example:

```bash
cargo run --example basic_usage
```

## Adding New Examples

To add a new example:

1. Create a new `.rs` file in this directory
2. Add the example to `Cargo.toml` under `[[example]]` section
3. Update this README with a description of the example

Example `Cargo.toml` entry:
```toml
[[example]]
name = "your_example_name"
path = "examples/your_example.rs"
```
