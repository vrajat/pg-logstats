# jq Query Patterns for pg-loggrep JSON Output

This document provides common jq patterns for analyzing pg-loggrep JSON output. These patterns help extract specific insights from the analysis results.

## Basic Usage

First, generate JSON output from pg-loggrep:
```bash
pg-loggrep --log-dir /path/to/logs --output-format json > analysis.json
```

Then use jq to query the results:
```bash
cat analysis.json | jq 'PATTERN'
```

## Common Patterns

### Summary Information

```bash
# Get basic summary statistics
jq '.summary' analysis.json

# Extract specific summary metrics
jq '.summary | {total_queries, avg_duration_ms, error_count}' analysis.json

# Check if there are performance issues
jq 'if .summary.avg_duration_ms > 100 then "Performance issues detected" else "Performance OK" end' analysis.json
```

### Query Analysis

```bash
# Get query distribution by type
jq '.query_analysis.by_type' analysis.json

# Find the most common query type
jq '.query_analysis.by_type | to_entries | max_by(.value) | .key' analysis.json

# Get top 5 most frequent queries
jq '.query_analysis.most_frequent[:5]' analysis.json

# Find queries with specific patterns
jq '.query_analysis.most_frequent[] | select(.query | contains("SELECT"))' analysis.json
```

### Performance Analysis

```bash
# Get all slow queries
jq '.query_analysis.slowest_queries' analysis.json

# Find queries slower than 1 second
jq '.query_analysis.slowest_queries[] | select(.duration_ms > 1000)' analysis.json

# Get average duration of slow queries
jq '[.query_analysis.slowest_queries[].duration_ms] | add / length' analysis.json

# Find queries with high frequency and duration
jq '.query_analysis.most_frequent[] | select(.count > 10 and .avg_duration_ms > 100)' analysis.json
```

### Error Analysis

```bash
# Check if errors exist (when error analysis is implemented)
jq 'has("error_summary")' analysis.json

# Get error count from summary
jq '.summary.error_count' analysis.json

# Alert if error rate is high
jq 'if (.summary.error_count / .summary.total_queries) > 0.1 then "High error rate!" else "Error rate OK" end' analysis.json
```

### Metadata and File Information

```bash
# Get analysis metadata
jq '.metadata' analysis.json

# List processed log files
jq '.metadata.log_files_processed[]' analysis.json

# Get analysis timestamp
jq '.metadata.analysis_timestamp' analysis.json

# Check tool version
jq '.metadata.tool_version' analysis.json
```

## Advanced Patterns

### Query Normalization Analysis

```bash
# Group similar queries (already normalized in output)
jq '.query_analysis.most_frequent | group_by(.query) | map({query: .[0].query, total_count: map(.count) | add})' analysis.json

# Find queries that might need optimization
jq '.query_analysis.most_frequent[] | select(.count > 100 or .avg_duration_ms > 500) | {query: .query, issue: (if .count > 100 then "high_frequency" else "slow_execution" end)}' analysis.json
```

### Performance Trending

```bash
# Calculate performance metrics
jq '{
  total_execution_time: (.summary.total_duration_ms),
  average_query_time: (.summary.avg_duration_ms),
  queries_per_second: (.summary.total_queries / (.summary.total_duration_ms / 1000)),
  performance_grade: (if .summary.avg_duration_ms < 10 then "A" elif .summary.avg_duration_ms < 50 then "B" elif .summary.avg_duration_ms < 100 then "C" else "D" end)
}' analysis.json
```

### Custom Reports

```bash
# Generate a performance report
jq '{
  report_date: .metadata.analysis_timestamp,
  summary: {
    total_queries: .summary.total_queries,
    avg_duration: .summary.avg_duration_ms,
    error_rate: (.summary.error_count / .summary.total_queries * 100)
  },
  top_queries: .query_analysis.most_frequent[:3],
  slow_queries: [.query_analysis.slowest_queries[] | select(.duration_ms > 100)],
  recommendations: [
    (if .summary.avg_duration_ms > 100 then "Consider query optimization" else empty end),
    (if (.summary.error_count / .summary.total_queries) > 0.05 then "Investigate error patterns" else empty end),
    (if .summary.total_queries > 10000 then "Consider connection pooling" else empty end)
  ]
}' analysis.json
```

### Filtering and Searching

```bash
# Find queries containing specific tables
jq '.query_analysis.most_frequent[] | select(.query | test("users|orders|products"; "i"))' analysis.json

# Get queries by type
jq '.query_analysis.most_frequent[] | select(.query | startswith("SELECT"))' analysis.json

# Find INSERT/UPDATE/DELETE queries (DML operations)
jq '.query_analysis.most_frequent[] | select(.query | test("^(INSERT|UPDATE|DELETE)"; "i"))' analysis.json
```

### Comparison and Benchmarking

```bash
# Compare query types distribution
jq '.query_analysis.by_type | to_entries | map({type: .key, count: .value, percentage: (.value / (.query_analysis.by_type | add) * 100)})' analysis.json

# Identify outliers in query performance
jq '
  (.query_analysis.most_frequent | map(.avg_duration_ms) | add / length) as $avg |
  .query_analysis.most_frequent[] |
  select(.avg_duration_ms > ($avg * 2)) |
  {query: .query, duration: .avg_duration_ms, deviation: (.avg_duration_ms / $avg)}
' analysis.json
```

## Output Formatting

### Pretty Printing

```bash
# Pretty print with colors (if terminal supports it)
jq -C '.' analysis.json

# Compact output
jq -c '.summary' analysis.json

# Raw output (no quotes for strings)
jq -r '.metadata.analysis_timestamp' analysis.json
```

### CSV Export

```bash
# Export most frequent queries to CSV format
jq -r '.query_analysis.most_frequent[] | [.count, .avg_duration_ms, .query] | @csv' analysis.json

# Export summary to CSV
jq -r '[.summary.total_queries, .summary.avg_duration_ms, .summary.error_count] | @csv' analysis.json
```

## Combining with Shell Commands

### Monitoring and Alerting

```bash
# Check for performance degradation
if [ $(jq '.summary.avg_duration_ms' analysis.json) -gt 100 ]; then
  echo "Performance alert: Average query time exceeds 100ms"
fi

# Count slow queries
slow_count=$(jq '[.query_analysis.slowest_queries[] | select(.duration_ms > 1000)] | length' analysis.json)
echo "Found $slow_count queries slower than 1 second"

# Generate alert if error rate is high
error_rate=$(jq '(.summary.error_count / .summary.total_queries * 100)' analysis.json)
if (( $(echo "$error_rate > 5" | bc -l) )); then
  echo "High error rate detected: ${error_rate}%"
fi
```

### Batch Processing

```bash
# Process multiple analysis files
for file in analysis_*.json; do
  echo "Processing $file:"
  jq '.summary | {file: "'$file'", queries: .total_queries, avg_duration: .avg_duration_ms}' "$file"
done

# Compare analyses over time
jq -s 'map({timestamp: .metadata.analysis_timestamp, avg_duration: .summary.avg_duration_ms}) | sort_by(.timestamp)' analysis_*.json
```

## Tips and Best Practices

1. **Use `jq -e` for error handling**: Exit with non-zero status if the filter produces no output
2. **Combine filters**: Use `|` to chain operations for complex queries
3. **Use variables**: Store intermediate results with `as $var` for complex calculations
4. **Test incrementally**: Build complex queries step by step
5. **Use `--tab` for TSV output**: Better for importing into spreadsheets
6. **Escape special characters**: Use proper escaping in regex patterns

## Common Issues and Solutions

### Empty Results
```bash
# Check if data exists before querying
jq 'if .query_analysis.most_frequent | length > 0 then .query_analysis.most_frequent[0] else "No queries found" end' analysis.json
```

### Type Errors
```bash
# Handle potential null values
jq '.summary.avg_duration_ms // 0' analysis.json

# Convert strings to numbers safely
jq '.summary.total_queries | tonumber' analysis.json
```

### Large Datasets
```bash
# Limit output size
jq '.query_analysis.most_frequent[:10]' analysis.json

# Stream processing for very large files
jq --stream '. as [$path, $value] | select($path[0] == "summary")' analysis.json
```

This guide covers the most common patterns for analyzing pg-loggrep JSON output. Combine these patterns to create custom analysis workflows that fit your specific monitoring and optimization needs.
