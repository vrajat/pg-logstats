# Analysis Recipes for pg-loggrep

This document provides step-by-step workflows for common PostgreSQL log analysis scenarios using pg-loggrep.

## Table of Contents

- [Performance Troubleshooting](#performance-troubleshooting)
- [Error Investigation](#error-investigation)
- [Capacity Planning](#capacity-planning)
- [Security Auditing](#security-auditing)
- [Query Optimization](#query-optimization)
- [Monitoring and Alerting](#monitoring-and-alerting)
- [Batch Analysis Workflows](#batch-analysis-workflows)

## Performance Troubleshooting

### Recipe 1: Identifying Slow Queries

**Scenario**: Application performance has degraded, need to find slow queries.

**Steps**:
1. Generate detailed analysis:
   ```bash
   pg-loggrep --log-dir /var/log/postgresql --output-format json > perf_analysis.json
   ```

2. Extract slow queries:
   ```bash
   jq '.query_analysis.slowest_queries[] | select(.duration_ms > 1000)' perf_analysis.json
   ```

3. Get top 10 slowest queries with context:
   ```bash
   jq '.query_analysis.slowest_queries[:10] | map({
     duration: .duration_ms,
     query: .query,
     frequency: .count,
     impact_score: (.duration_ms * .count)
   }) | sort_by(-.impact_score)' perf_analysis.json
   ```

4. Generate optimization recommendations:
   ```bash
   jq '.query_analysis.slowest_queries[] |
   select(.duration_ms > 1000) |
   {
     query: .query,
     duration: .duration_ms,
     recommendation: (
       if (.query | test("WHERE.*=.*AND"; "i")) then "Consider composite index"
       elif (.query | test("ORDER BY"; "i")) then "Check if ORDER BY columns are indexed"
       elif (.query | test("JOIN"; "i")) then "Verify JOIN conditions are indexed"
       else "Review query execution plan"
       end
     )
   }' perf_analysis.json
   ```

### Recipe 2: Analyzing Query Patterns

**Scenario**: Understanding overall query distribution and patterns.

**Steps**:
1. Get query type distribution:
   ```bash
   pg-loggrep --log-dir /var/log/postgresql --output-format json | \
   jq '.query_analysis.by_type | to_entries | map({
     type: .key,
     count: .value,
     percentage: (.value / (.query_analysis.by_type | add) * 100 | round)
   }) | sort_by(-.count)'
   ```

2. Identify read vs write patterns:
   ```bash
   pg-loggrep --log-dir /var/log/postgresql --output-format json | \
   jq '{
     read_queries: (.query_analysis.by_type.SELECT // 0),
     write_queries: ((.query_analysis.by_type.INSERT // 0) + (.query_analysis.by_type.UPDATE // 0) + (.query_analysis.by_type.DELETE // 0)),
     ddl_queries: (.query_analysis.by_type.DDL // 0),
     other_queries: (.query_analysis.by_type.OTHER // 0)
   } | . + {
     read_write_ratio: (.read_queries / (.write_queries + 1)),
     workload_type: (if .read_write_ratio > 10 then "read_heavy" elif .read_write_ratio < 0.1 then "write_heavy" else "balanced" end)
   }'
   ```

## Error Investigation

### Recipe 3: Error Pattern Analysis

**Scenario**: Investigating recurring errors in the application.

**Steps**:
1. Quick error overview:
   ```bash
   pg-loggrep --log-dir /var/log/postgresql --output-format text | grep -A 5 -B 5 "Error Count:"
   ```

2. Detailed error analysis (when error analysis is implemented):
   ```bash
   pg-loggrep --log-dir /var/log/postgresql --output-format json | \
   jq 'if has("error_summary") then .error_summary else "Error analysis not available in current version" end'
   ```

3. Find queries that might be causing errors:
   ```bash
   # Look for common error patterns in query text
   pg-loggrep --log-dir /var/log/postgresql --output-format json | \
   jq '.query_analysis.most_frequent[] |
   select(.query | test("\\$[0-9]+|\\?|--|/\\*|;.*--|UNION.*SELECT"; "i")) |
   {query: .query, count: .count, risk: "potential_sql_injection_or_syntax_error"}'
   ```

### Recipe 4: Connection Issues

**Scenario**: Investigating connection problems and authentication failures.

**Steps**:
1. Analyze connection patterns:
   ```bash
   pg-loggrep --log-dir /var/log/postgresql --output-format json | \
   jq '.summary.connection_count'
   ```

2. Look for authentication patterns in logs:
   ```bash
   # This would require enhanced log parsing for connection events
   grep -E "(connection received|connection authorized|authentication failed)" /var/log/postgresql/*.log | \
   head -20
   ```

## Capacity Planning

### Recipe 5: Workload Analysis

**Scenario**: Planning for database capacity and resource allocation.

**Steps**:
1. Analyze query volume and timing:
   ```bash
   pg-loggrep --log-dir /var/log/postgresql --output-format json | \
   jq '{
     total_queries: .summary.total_queries,
     avg_duration: .summary.avg_duration_ms,
     total_time: .summary.total_duration_ms,
     estimated_qps: (.summary.total_queries / (.summary.total_duration_ms / 1000)),
     performance_profile: {
       fast_queries: [.query_analysis.most_frequent[] | select(.avg_duration_ms < 10)] | length,
       medium_queries: [.query_analysis.most_frequent[] | select(.avg_duration_ms >= 10 and .avg_duration_ms < 100)] | length,
       slow_queries: [.query_analysis.most_frequent[] | select(.avg_duration_ms >= 100)] | length
     }
   }'
   ```

2. Resource utilization estimation:
   ```bash
   pg-loggrep --log-dir /var/log/postgresql --output-format json | \
   jq '{
     peak_concurrent_estimate: (.summary.total_queries / 3600),
     memory_pressure_indicators: [.query_analysis.most_frequent[] | select(.query | test("ORDER BY|GROUP BY|JOIN"; "i"))] | length,
     io_intensive_queries: [.query_analysis.most_frequent[] | select(.avg_duration_ms > 100)] | length
   }'
   ```

## Security Auditing

### Recipe 6: Security Pattern Detection

**Scenario**: Auditing for potential security issues in query patterns.

**Steps**:
1. Detect potential SQL injection patterns:
   ```bash
   pg-loggrep --log-dir /var/log/postgresql --output-format json | \
   jq '.query_analysis.most_frequent[] |
   select(.query | test("(\\$[0-9]+.*\\$[0-9]+|--|/\\*.*\\*/|UNION.*SELECT|OR.*1.*=.*1)"; "i")) |
   {
     query: .query,
     count: .count,
     risk_level: "high",
     pattern: "potential_sql_injection"
   }'
   ```

2. Find queries with dynamic content:
   ```bash
   pg-loggrep --log-dir /var/log/postgresql --output-format json | \
   jq '.query_analysis.most_frequent[] |
   select(.query | test("(LIKE.*%|IN.*\\(.*,.*\\)|BETWEEN.*AND)"; "i")) |
   {query: .query, count: .count, note: "dynamic_content_detected"}'
   ```

## Query Optimization

### Recipe 7: Index Optimization Analysis

**Scenario**: Identifying queries that would benefit from indexing.

**Steps**:
1. Find queries with WHERE clauses that might need indexes:
   ```bash
   pg-loggrep --log-dir /var/log/postgresql --output-format json | \
   jq '.query_analysis.most_frequent[] |
   select(.count > 10 and (.query | test("WHERE.*="; "i"))) |
   {
     query: .query,
     frequency: .count,
     avg_duration: .avg_duration_ms,
     index_candidate: (.query | capture("WHERE\\s+(?<column>\\w+)\\s*="; "i").column // "unknown")
   } | select(.avg_duration > 10)'
   ```

2. Identify JOIN optimization opportunities:
   ```bash
   pg-loggrep --log-dir /var/log/postgresql --output-format json | \
   jq '.query_analysis.most_frequent[] |
   select(.avg_duration_ms > 50 and (.query | test("JOIN"; "i"))) |
   {
     query: .query,
     duration: .avg_duration_ms,
     frequency: .count,
     optimization_priority: (.avg_duration_ms * .count),
     suggestion: "Review JOIN conditions and ensure proper indexing"
   } | select(.optimization_priority > 1000)'
   ```

### Recipe 8: Query Normalization Review

**Scenario**: Finding similar queries that could be parameterized.

**Steps**:
1. Group similar query patterns:
   ```bash
   pg-loggrep --log-dir /var/log/postgresql --output-format json | \
   jq '.query_analysis.most_frequent |
   group_by(.query | gsub("[0-9]+"; "N") | gsub("'"'"'[^'"'"']*'"'"'"; "S")) |
   map(select(length > 1) | {
     pattern: .[0].query,
     variations: length,
     total_count: map(.count) | add,
     avg_duration: (map(.avg_duration_ms * .count) | add) / (map(.count) | add)
   }) | sort_by(-.total_count)'
   ```

## Monitoring and Alerting

### Recipe 9: Performance Monitoring Setup

**Scenario**: Setting up automated performance monitoring.

**Steps**:
1. Create performance baseline:
   ```bash
   #!/bin/bash
   # baseline.sh
   pg-loggrep --log-dir /var/log/postgresql --output-format json > baseline_$(date +%Y%m%d).json

   # Extract key metrics
   jq '{
     timestamp: now,
     avg_duration: .summary.avg_duration_ms,
     total_queries: .summary.total_queries,
     error_rate: (.summary.error_count / .summary.total_queries * 100),
     slow_query_count: [.query_analysis.slowest_queries[] | select(.duration_ms > 1000)] | length
   }' baseline_$(date +%Y%m%d).json >> performance_baseline.jsonl
   ```

2. Performance alerting script:
   ```bash
   #!/bin/bash
   # alert.sh
   CURRENT=$(pg-loggrep --log-dir /var/log/postgresql --output-format json --quick)
   AVG_DURATION=$(echo "$CURRENT" | jq '.summary.avg_duration_ms')
   ERROR_RATE=$(echo "$CURRENT" | jq '(.summary.error_count / .summary.total_queries * 100)')

   if (( $(echo "$AVG_DURATION > 100" | bc -l) )); then
     echo "ALERT: Average query duration is ${AVG_DURATION}ms (threshold: 100ms)"
   fi

   if (( $(echo "$ERROR_RATE > 5" | bc -l) )); then
     echo "ALERT: Error rate is ${ERROR_RATE}% (threshold: 5%)"
   fi
   ```

### Recipe 10: Trend Analysis

**Scenario**: Analyzing performance trends over time.

**Steps**:
1. Collect daily metrics:
   ```bash
   #!/bin/bash
   # daily_metrics.sh
   DATE=$(date +%Y-%m-%d)
   pg-loggrep --log-dir /var/log/postgresql --output-format json | \
   jq --arg date "$DATE" '{
     date: $date,
     metrics: {
       total_queries: .summary.total_queries,
       avg_duration: .summary.avg_duration_ms,
       error_count: .summary.error_count,
       slow_queries: [.query_analysis.slowest_queries[] | select(.duration_ms > 1000)] | length,
       top_query_types: .query_analysis.by_type
     }
   }' >> daily_metrics.jsonl
   ```

2. Generate trend report:
   ```bash
   # Analyze last 7 days of metrics
   tail -7 daily_metrics.jsonl | jq -s '
   {
     period: "last_7_days",
     trend_analysis: {
       avg_duration_trend: (.[6].metrics.avg_duration - .[0].metrics.avg_duration),
       query_volume_trend: (.[6].metrics.total_queries - .[0].metrics.total_queries),
       error_trend: (.[6].metrics.error_count - .[0].metrics.error_count),
       performance_stability: (map(.metrics.avg_duration) | (max - min))
     },
     recommendations: [
       (if (.[6].metrics.avg_duration - .[0].metrics.avg_duration) > 10 then "Performance degrading - investigate slow queries" else empty end),
       (if (.[6].metrics.error_count - .[0].metrics.error_count) > 0 then "Error count increasing - check application logs" else empty end)
     ]
   }'
   ```

## Batch Analysis Workflows

### Recipe 11: Multi-Day Analysis

**Scenario**: Analyzing patterns across multiple days of logs.

**Steps**:
1. Process multiple log directories:
   ```bash
   #!/bin/bash
   # multi_day_analysis.sh
   for day in {1..7}; do
     date_dir="/var/log/postgresql/$(date -d "$day days ago" +%Y-%m-%d)"
     if [ -d "$date_dir" ]; then
       echo "Processing $date_dir..."
       pg-loggrep --log-dir "$date_dir" --output-format json > "analysis_$(date -d "$day days ago" +%Y%m%d).json"
     fi
   done
   ```

2. Combine and compare results:
   ```bash
   jq -s 'map({
     date: (.metadata.analysis_timestamp | split("T")[0]),
     summary: .summary,
     top_queries: .query_analysis.most_frequent[:5]
   }) | sort_by(.date)' analysis_*.json > weekly_summary.json
   ```

### Recipe 12: Automated Reporting

**Scenario**: Generate automated daily/weekly reports.

**Steps**:
1. Daily report generator:
   ```bash
   #!/bin/bash
   # daily_report.sh
   DATE=$(date +%Y-%m-%d)
   REPORT_FILE="daily_report_${DATE}.md"

   echo "# PostgreSQL Performance Report - $DATE" > "$REPORT_FILE"
   echo "" >> "$REPORT_FILE"

   # Generate analysis
   ANALYSIS=$(pg-loggrep --log-dir /var/log/postgresql --output-format json)

   # Summary section
   echo "## Summary" >> "$REPORT_FILE"
   echo "$ANALYSIS" | jq -r '"- Total Queries: " + (.summary.total_queries | tostring)' >> "$REPORT_FILE"
   echo "$ANALYSIS" | jq -r '"- Average Duration: " + (.summary.avg_duration_ms | tostring) + "ms"' >> "$REPORT_FILE"
   echo "$ANALYSIS" | jq -r '"- Error Count: " + (.summary.error_count | tostring)' >> "$REPORT_FILE"
   echo "" >> "$REPORT_FILE"

   # Top queries section
   echo "## Top 5 Most Frequent Queries" >> "$REPORT_FILE"
   echo "$ANALYSIS" | jq -r '.query_analysis.most_frequent[:5][] | "- **" + (.count | tostring) + "x**: " + .query' >> "$REPORT_FILE"

   echo "Report generated: $REPORT_FILE"
   ```

These recipes provide practical workflows for common PostgreSQL log analysis scenarios. Adapt them to your specific environment and requirements by modifying the thresholds, patterns, and output formats as needed.
