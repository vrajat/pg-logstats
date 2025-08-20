# Testing Guide for pg-logstats

This document provides comprehensive instructions for running tests in various configurations for the pg-logstats PostgreSQL log analysis tool.

## Test Structure

The test suite is organized into several categories:

### 1. Unit Tests (`tests/unit/`)
- **Parser Tests** (`parser_tests.rs`): Tests for PostgreSQL stderr log parsing
- **Analytics Tests** (`analytics_tests.rs`): Tests for query analysis and metrics calculation
- **Output Tests** (`output_tests.rs`): Tests for text and JSON output formatting

### 2. Integration Tests (`tests/integration_tests.rs`)
- End-to-end CLI testing with sample log files
- Docker environment testing
- Error handling scenarios
- Performance benchmarks

### 3. Test Data (`tests/test_data/`)
- Utilities for generating various types of test log files
- Expected output files for validation
- Edge cases including empty files, malformed lines, and large files

## Running Tests

### Basic Test Execution

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run specific test module
cargo test parser_tests
cargo test analytics_tests
cargo test output_tests
cargo test integration_tests

# Run tests matching a pattern
cargo test test_parse_simple_statement
```

### Unit Tests

```bash
# Run all unit tests
cargo test --test parser_tests
cargo test --test analytics_tests
cargo test --test output_tests

# Run specific unit test categories
cargo test parser_unit_tests
cargo test analytics_unit_tests
cargo test text_formatter_tests
cargo test json_formatter_tests
```

### Integration Tests

```bash
# Run all integration tests
cargo test --test integration_tests

# Run specific integration test categories
cargo test cli_basic_tests
cargo test file_processing_tests
cargo test error_handling_tests
cargo test performance_tests
```

### Property-Based Tests

```bash
# Run property-based tests
cargo test property_based_tests
cargo test property_based_analytics_tests
```

### Performance and Benchmark Tests

```bash
# Run performance tests
cargo test performance_tests

# Run with release mode for accurate performance measurements
cargo test --release performance_tests

# Run memory usage tests
cargo test memory_usage

# Run benchmark tests
cargo test benchmark
```

### Docker Environment Tests

```bash
# Run Docker-related tests (requires Docker)
cargo test docker_tests

# Run with Docker environment setup
./demo/scripts/setup.sh
cargo test docker_tests
./demo/scripts/cleanup.sh
```

## Test Configuration

### Environment Variables

```bash
# Enable debug logging during tests
RUST_LOG=debug cargo test

# Set custom test timeout
RUST_TEST_TIME_UNIT=60000 cargo test

# Run tests in single thread (useful for debugging)
cargo test -- --test-threads=1
```

### Test Data Generation

The test suite includes utilities for generating various types of test data:

```bash
# Generate test data (done automatically during tests)
cargo test generate_test_data

# Test with large datasets
cargo test test_performance_with_large_dataset

# Test with edge cases
cargo test edge_case_tests
```

## Continuous Integration

### GitHub Actions / CI Pipeline

```yaml
# Example CI configuration
name: Tests
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Run tests
        run: |
          cargo test --verbose
          cargo test --release --verbose
```

### Test Coverage

```bash
# Install cargo-tarpaulin for coverage
cargo install cargo-tarpaulin

# Generate coverage report
cargo tarpaulin --out Html --output-dir coverage/

# Generate coverage with specific test patterns
cargo tarpaulin --tests parser_tests analytics_tests output_tests
```

## Test Categories and Best Practices

### 1. Parser Tests
- **Log Line Formats**: Tests various PostgreSQL log line formats
- **Edge Cases**: Empty lines, malformed entries, continuation lines
- **Multi-line Statements**: Complex queries spanning multiple lines
- **Timestamp Parsing**: Different timestamp formats and timezones
- **Error Handling**: Invalid log entries and parsing failures

### 2. Analytics Tests
- **Query Classification**: SELECT, INSERT, UPDATE, DELETE, DDL, OTHER
- **Query Normalization**: Parameter replacement, literal normalization
- **Performance Metrics**: Duration calculations, percentiles
- **Frequency Analysis**: Most frequent and slowest queries
- **Error Rate Calculation**: Error counting and rate calculation

### 3. Output Tests
- **Text Formatting**: Human-readable output with colors
- **JSON Formatting**: Structured output with metadata
- **Edge Cases**: Special characters, Unicode, very long queries
- **Performance**: Large dataset formatting performance

### 4. Integration Tests
- **CLI Interface**: Command-line argument parsing and validation
- **File Processing**: Single files, multiple files, directories
- **Output Formats**: Text and JSON output validation
- **Error Scenarios**: Missing files, invalid arguments
- **Performance**: Large file processing benchmarks

## Debugging Tests

### Running Individual Tests

```bash
# Run a specific test with debug output
cargo test test_parse_simple_statement -- --nocapture

# Run tests with backtrace on panic
RUST_BACKTRACE=1 cargo test

# Run tests with full backtrace
RUST_BACKTRACE=full cargo test
```

### Test Debugging Tips

1. **Use `println!` or `dbg!`** for debugging test values
2. **Check test data** in `tests/test_data/` for expected inputs
3. **Verify file paths** when tests fail with file not found errors
4. **Check permissions** for file creation/deletion tests
5. **Use `--nocapture`** to see test output

## Performance Testing

### Benchmark Configuration

```bash
# Run performance tests in release mode
cargo test --release performance_tests

# Run with specific performance thresholds
cargo test test_performance_with_large_dataset

# Memory usage validation
cargo test test_memory_usage
```

### Performance Thresholds

The test suite includes performance assertions:
- **Parser Performance**: < 1000ms for 1000 log lines
- **Analytics Performance**: < 1000ms for 1000 queries
- **Output Formatting**: < 1000ms for large datasets
- **Memory Usage**: Reasonable memory consumption for large inputs

## Mocking and Test Doubles

### External Dependencies

The test suite uses mocking for:
- **File System Operations**: Using `tempfile` for temporary directories
- **Time-based Tests**: Fixed timestamps for reproducible results
- **External Commands**: Mocked CLI interactions

### Test Data Management

- **Temporary Files**: Automatically cleaned up after tests
- **Deterministic Data**: Fixed seeds for reproducible test data
- **Edge Case Coverage**: Comprehensive edge case scenarios

## Contributing to Tests

### Adding New Tests

1. **Unit Tests**: Add to appropriate module in `tests/unit/`
2. **Integration Tests**: Add to `tests/integration_tests.rs`
3. **Test Data**: Add generators to `tests/test_data/mod.rs`
4. **Documentation**: Update this README with new test categories

### Test Naming Conventions

- **Unit Tests**: `test_function_name_scenario`
- **Integration Tests**: `test_cli_feature_scenario`
- **Property Tests**: `property_description`
- **Performance Tests**: `test_performance_scenario`

### Test Organization

- **Group related tests** in modules
- **Use descriptive test names** that explain the scenario
- **Include both positive and negative test cases**
- **Test edge cases and error conditions**
- **Validate both success and failure paths**

## Troubleshooting

### Common Test Failures

1. **File Not Found**: Check test data generation and paths
2. **Permission Denied**: Ensure test has write permissions
3. **Timeout**: Increase timeout for performance tests
4. **Assertion Failures**: Check expected vs actual values
5. **Docker Issues**: Ensure Docker is running for Docker tests

### Getting Help

- Check test output with `--nocapture`
- Use `RUST_LOG=debug` for detailed logging
- Review test data in `tests/test_data/`
- Check CI logs for environment-specific issues
- Run tests individually to isolate problems

## Test Metrics

The test suite aims for:
- **Code Coverage**: > 90%
- **Test Execution Time**: < 30 seconds for full suite
- **Performance Regression**: No degradation in benchmark tests
- **Memory Usage**: Stable memory consumption patterns
- **Error Coverage**: All error paths tested
