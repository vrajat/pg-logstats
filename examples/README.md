# pg-loggrep Examples

This directory contains usage examples for the pg-loggrep library.

## Examples

### basic_usage.rs

A basic example showing how to:
- Parse PostgreSQL log files
- Analyze queries and timing
- Format results in different output formats (JSON and text)

To run this example:

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
