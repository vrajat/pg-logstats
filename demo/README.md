# pg-logstats Demo

This directory contains a complete demonstration environment for pg-logstats, including Docker-based PostgreSQL setup, sample workloads, and step-by-step tutorials.

## Table of Contents

- [Quick Start](#quick-start)
- [Demo Components](#demo-components)
- [Step-by-Step Walkthrough](#step-by-step-walkthrough)
- [Docker Environment](#docker-environment)
- [Sample Workloads](#sample-workloads)
- [Analysis Examples](#analysis-examples)
- [Troubleshooting](#troubleshooting)
- [Customization](#customization)

## Quick Start

Get up and running with pg-logstats in under 5 minutes:

```bash
# 1. Start the demo environment
cd demo
./scripts/setup.sh

# 2. Run the complete demo
./scripts/run-demo.sh

# 3. View the results
ls -la analysis_output/
```

## Demo Components

### Directory Structure

```
demo/
├── README.md                    # This file
├── docker/                     # Docker environment
│   ├── docker-compose.yml      # PostgreSQL + workload containers
│   ├── README.md               # Docker setup documentation
│   ├── postgres/               # PostgreSQL configuration
│   │   ├── Dockerfile          # Custom PostgreSQL image
│   │   └── postgresql.conf     # Optimized for logging
│   └── workload/               # Workload generator
│       ├── Dockerfile          # Workload container
│       └── minimal_workload.sql # Sample SQL workload
└── scripts/                    # Automation scripts
    ├── setup.sh                # Environment setup
    ├── run-demo.sh             # Complete demo runner
    ├── run-workload.sh         # Workload execution
    ├── docker-demo.sh          # Docker-specific demo
    └── cleanup.sh              # Environment cleanup
```

### Key Features

- **Containerized Environment**: Complete PostgreSQL setup with Docker
- **Realistic Workloads**: Mixed OLTP and analytical query patterns
- **Automated Analysis**: Scripts for running pg-logstats analysis
- **Multiple Output Formats**: Text, JSON, and custom reports
- **Performance Monitoring**: Real-time log analysis examples

## Step-by-Step Walkthrough

### Phase 1: Environment Setup

1. **Prerequisites Check**
   ```bash
   # Ensure required tools are installed
   docker --version
   docker-compose --version
   cargo --version  # For building pg-logstats
   ```

2. **Build pg-logstats**
   ```bash
   # From the project root
   cargo build --release

   # Verify installation
   ./target/release/pg-logstats --version
   ```

3. **Setup Demo Environment**
   ```bash
   cd demo
   ./scripts/setup.sh
   ```

   This script will:
   - Create necessary directories
   - Build Docker images
   - Initialize PostgreSQL with sample data
   - Configure logging for optimal analysis

### Phase 2: Generate Sample Data

1. **Start PostgreSQL Container**
   ```bash
   ./scripts/docker-demo.sh start
   ```

2. **Run Sample Workload**
   ```bash
   ./scripts/run-workload.sh --duration 300 --connections 5
   ```

   This generates:
   - Mixed query patterns (SELECT, INSERT, UPDATE, DELETE)
   - Varying query complexities
   - Realistic timing patterns
   - Error scenarios for testing

3. **Monitor Log Generation**
   ```bash
   # Watch logs being generated
   docker-compose -f docker/docker-compose.yml logs -f postgres

   # Check log file size
   docker exec demo-postgres ls -lh /var/log/postgresql/
   ```

### Phase 3: Basic Analysis

1. **Simple Analysis**
   ```bash
   # Analyze generated logs
   ./target/release/pg-logstats \
     --log-dir ./demo/logs \
     --output-format text
   ```

2. **JSON Output for Processing**
   ```bash
   ./target/release/pg-logstats \
     --log-dir ./demo/logs \
     --output-format json \
     > analysis_output/basic_analysis.json
   ```

3. **Quick Summary Mode**
   ```bash
   ./target/release/pg-logstats \
     --log-dir ./demo/logs \
     --quick
   ```

### Phase 4: Advanced Analysis

1. **Performance Focus**
   ```bash
   # Analyze with performance focus
   ./target/release/pg-logstats \
     --log-dir ./demo/logs \
     --output-format json | \
     jq '.query_analysis.slowest_queries[]'
   ```

2. **Error Analysis**
   ```bash
   # Focus on errors and warnings
   ./target/release/pg-logstats \
     --log-dir ./demo/logs \
     --output-format json | \
     jq 'select(.summary.error_count > 0)'
   ```

3. **Query Pattern Analysis**
   ```bash
   # Analyze query patterns
   ./target/release/pg-logstats \
     --log-dir ./demo/logs \
     --output-format json | \
     jq '.query_analysis.by_type'
   ```

### Phase 5: Batch Processing

1. **Multi-Day Analysis**
   ```bash
   # Use the batch processing script
   ../examples/integration/batch_processing.sh \
     multi-day \
     --log-dir ./demo/logs \
     --days 7 \
     --parallel 4
   ```

2. **Trend Analysis**
   ```bash
   ../examples/integration/batch_processing.sh \
     trend-analysis \
     --days 7
   ```

3. **Automated Reporting**
   ```bash
   ../examples/integration/batch_processing.sh \
     weekly-summary \
     --days 7
   ```

## Docker Environment

### Starting the Environment

```bash
# Start PostgreSQL and workload containers
cd demo
docker-compose -f docker/docker-compose.yml up -d

# Verify containers are running
docker-compose -f docker/docker-compose.yml ps
```

### PostgreSQL Configuration

The demo PostgreSQL instance is configured for optimal logging:

```sql
-- Key logging settings
log_destination = 'stderr'
log_statement = 'all'
log_duration = on
log_min_duration_statement = 0
log_line_prefix = '%t [%p]: [%l-1] user=%u,db=%d '
```

### Accessing the Database

```bash
# Connect to PostgreSQL
docker exec -it demo-postgres psql -U postgres -d demo

# Run sample queries
\dt  -- List tables
SELECT COUNT(*) FROM users;
SELECT * FROM orders LIMIT 5;
```

### Log File Access

```bash
# View current logs
docker exec demo-postgres tail -f /var/log/postgresql/postgresql.log

# Copy logs to host for analysis
docker cp demo-postgres:/var/log/postgresql/ ./demo/logs/
```

## Sample Workloads

### Minimal Workload

Basic CRUD operations for testing:

```sql
-- Create sample data
INSERT INTO users (name, email) VALUES ('John Doe', 'john@example.com');
INSERT INTO orders (user_id, total) VALUES (1, 99.99);

-- Query patterns
SELECT * FROM users WHERE id = 1;
SELECT COUNT(*) FROM orders;
UPDATE users SET last_login = NOW() WHERE id = 1;
```

### Realistic Workload

Mixed workload simulating real application usage:

```bash
./scripts/run-workload.sh --workload realistic --duration 600
```

This includes:
- **OLTP Queries**: Fast, frequent operations
- **Analytical Queries**: Complex aggregations and joins
- **Batch Operations**: Large data modifications
- **Error Scenarios**: Invalid queries and constraint violations

### Performance Testing Workload

Stress testing with various query complexities:

```bash
./scripts/run-workload.sh --workload performance --connections 10
```

Features:
- **Concurrent Connections**: Multiple simultaneous users
- **Query Variations**: Different complexity levels
- **Resource Intensive**: Memory and CPU intensive operations
- **Long-Running Queries**: Queries with significant duration

## Analysis Examples

### Example 1: Daily Performance Report

```bash
# Generate daily report
./target/release/pg-logstats \
  --log-dir ./demo/logs \
  --output-format json | \
  jq '{
    date: .metadata.analysis_timestamp,
    summary: .summary,
    top_queries: .query_analysis.most_frequent[:5],
    slow_queries: [.query_analysis.slowest_queries[] | select(.duration_ms > 100)]
  }' > daily_report.json
```

### Example 2: Error Investigation

```bash
# Find and analyze errors
./target/release/pg-logstats \
  --log-dir ./demo/logs \
  --output-format json | \
  jq 'select(.summary.error_count > 0) | {
    error_count: .summary.error_count,
    total_queries: .summary.total_queries,
    error_rate: (.summary.error_count / .summary.total_queries * 100)
  }'
```

### Example 3: Performance Optimization

```bash
# Identify optimization opportunities
./target/release/pg-logstats \
  --log-dir ./demo/logs \
  --output-format json | \
  jq '.query_analysis.most_frequent[] |
      select(.avg_duration_ms > 50 or .count > 100) |
      {
        query: .query,
        frequency: .count,
        avg_duration: .avg_duration_ms,
        optimization_priority: (.avg_duration_ms * .count)
      }' | \
  jq -s 'sort_by(-.optimization_priority)'
```

## Troubleshooting

### Common Issues

#### 1. Docker Container Won't Start

**Problem**: PostgreSQL container fails to start
```bash
docker-compose -f docker/docker-compose.yml logs postgres
```

**Solutions**:
- Check port 5432 availability: `lsof -i :5432`
- Verify Docker resources: `docker system df`
- Reset environment: `./scripts/cleanup.sh && ./scripts/setup.sh`

#### 2. No Log Files Generated

**Problem**: pg-logstats finds no log files
```bash
ls -la demo/logs/
```

**Solutions**:
- Verify PostgreSQL logging configuration
- Check log file permissions
- Ensure workload has been executed
- Verify log directory path

#### 3. Analysis Shows No Queries

**Problem**: pg-logstats reports zero queries
```bash
./target/release/pg-logstats --log-dir ./demo/logs --verbose
```

**Solutions**:
- Check log format compatibility
- Verify log file content: `head -20 demo/logs/postgresql.log`
- Ensure queries were actually executed
- Check for parsing errors in verbose output

#### 4. Performance Issues

**Problem**: Analysis takes too long
```bash
# Use sampling for large files
./target/release/pg-logstats \
  --log-dir ./demo/logs \
  --sample-size 10000
```

**Solutions**:
- Use `--sample-size` for large log files
- Enable `--quick` mode for summary only
- Process files in parallel with batch scripts
- Increase system resources for Docker

### Debug Mode

Enable detailed logging for troubleshooting:

```bash
# Enable debug logging
RUST_LOG=debug ./target/release/pg-logstats \
  --log-dir ./demo/logs \
  --verbose

# Check pg-logstats version and features
./target/release/pg-logstats --version
```

### Log Validation

Verify log format and content:

```bash
# Check log format
head -5 demo/logs/postgresql.log

# Validate timestamp format
grep -E "^\d{4}-\d{2}-\d{2}" demo/logs/postgresql.log | head -3

# Count total log lines
wc -l demo/logs/postgresql.log
```

## Customization

### Custom Workloads

Create your own workload patterns:

1. **Create SQL File**
   ```sql
   -- custom_workload.sql
   SELECT * FROM custom_table WHERE condition = 'value';
   INSERT INTO custom_table (col1, col2) VALUES ('val1', 'val2');
   -- Add more queries...
   ```

2. **Run Custom Workload**
   ```bash
   docker exec -i demo-postgres psql -U postgres -d demo < custom_workload.sql
   ```

### Custom Analysis Scripts

Extend the analysis with custom jq patterns:

```bash
# Create custom analysis script
cat > custom_analysis.sh << 'EOF'
#!/bin/bash
./target/release/pg-logstats \
  --log-dir ./demo/logs \
  --output-format json | \
  jq '{
    custom_metric: (.summary.total_queries / .summary.avg_duration_ms),
    query_efficiency: .query_analysis.by_type,
    performance_grade: (
      if .summary.avg_duration_ms < 10 then "A"
      elif .summary.avg_duration_ms < 50 then "B"
      elif .summary.avg_duration_ms < 100 then "C"
      else "D"
      end
    )
  }'
EOF

chmod +x custom_analysis.sh
./custom_analysis.sh
```

### Environment Configuration

Customize the demo environment:

1. **PostgreSQL Settings**
   Edit `docker/postgres/postgresql.conf`:
   ```conf
   # Custom logging settings
   log_min_duration_statement = 100  # Only log slow queries
   log_statement = 'mod'              # Only log modifications
   ```

2. **Workload Parameters**
   Edit `docker/workload/minimal_workload.sql`:
   ```sql
   -- Add your custom queries
   SELECT custom_function(param1, param2);
   ```

3. **Analysis Configuration**
   Create custom analysis profiles:
   ```bash
   # Performance-focused analysis
   alias pg-perf='./target/release/pg-logstats --output-format json | jq ".query_analysis.slowest_queries"'

   # Error-focused analysis
   alias pg-errors='./target/release/pg-logstats --output-format json | jq "select(.summary.error_count > 0)"'
   ```

### Integration with Monitoring

Connect pg-logstats with monitoring systems:

```bash
# Prometheus metrics export
./target/release/pg-logstats \
  --log-dir ./demo/logs \
  --output-format json | \
  jq -r '
    "# HELP pg_queries_total Total number of queries",
    "# TYPE pg_queries_total counter",
    "pg_queries_total " + (.summary.total_queries | tostring),
    "# HELP pg_avg_duration_ms Average query duration in milliseconds",
    "# TYPE pg_avg_duration_ms gauge",
    "pg_avg_duration_ms " + (.summary.avg_duration_ms | tostring)
  '
```

This comprehensive demo provides everything needed to understand and evaluate pg-logstats's capabilities in a realistic PostgreSQL environment.
