# pg-loggrep Documentation

This directory contains documentation for the pg-loggrep project.

## Documentation Files

### API.md

Complete API documentation for the pg-loggrep library, including:
- Module overview
- Data structures
- Method documentation
- Usage examples
- Error handling

## Building Documentation

To build the documentation locally:

```bash
cargo doc --no-deps --open
```

This will generate HTML documentation and open it in your default browser. The generated docs include the latest public API for:
- `parsers` (e.g., `StderrParser`)
- `analytics` (e.g., `QueryAnalyzer`, `TimingAnalyzer`)
- `output` (e.g., `JsonFormatter`, `TextFormatter`)

## Contributing to Documentation

When adding new features to pg-loggrep:

1. Update the relevant module documentation in the source code
2. Update this API.md file if new public APIs are added
3. Add usage examples to the `examples/` directory
4. Update the main README.md if necessary
