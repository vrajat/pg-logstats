# PostgreSQL Docker Demo Environment for pg-loggrep

This directory contains a complete Docker development environment for testing and demonstrating pg-loggrep with PostgreSQL 17. The environment includes a PostgreSQL server with comprehensive logging enabled and a configurable workload generator.

## Quick Start

### Start PostgreSQL Only
```bash
docker-compose up postgres
```

### Start PostgreSQL and Run Workload
```bash
docker-compose up
```

### Run Workload Separately
```bash
# Start PostgreSQL first
docker-compose up -d postgres

# Run workload with default settings
docker-compose run --rm workload

# Run workload with custom settings
docker-compose run --rm -e WORKLOAD_TYPE=intensive -e WORKLOAD_ITERATIONS=10 workload
```

## Services

### PostgreSQL Service (`postgres`)

- **Image**: PostgreSQL 17 with custom configuration
- **Port**: 5432 (mapped to host)
- **Database**: `testdb`
- **User**: `testuser`
- **Password**: `testpass`
- **Features**:
  - Comprehensive logging enabled (all statements, durations, connections)
  - pg_stat_statements extension for query statistics
  - Optimized for log analysis and demonstration
  - Named volume for persistent data
  - Shared log volume for analysis

### Workload Service (`workload`)

- **Image**: Alpine Linux with PostgreSQL client
- **Purpose**: Generate realistic database workload for log analysis
- **Features**:
  - Multiple workload types (basic, intensive, errors, mixed)
  - Configurable iterations and delays
  - Comprehensive SQL operations (DDL, DML, queries)
  - Error generation for testing error handling
  - Health check integration

## Configuration

### Environment Variables

#### PostgreSQL Service
- `POSTGRES_DB`: Database name (default: `testdb`)
- `POSTGRES_USER`: Database user (default: `testuser`)
- `POSTGRES_PASSWORD`: Database password (default: `testpass`)

#### Workload Service
- `POSTGRES_HOST`: PostgreSQL hostname (default: `postgres`)
- `POSTGRES_PORT`: PostgreSQL port (default: `5432`)
- `POSTGRES_DB`: Database name (default: `testdb`)
- `POSTGRES_USER`: Database user (default: `testuser`)
- `POSTGRES_PASSWORD`: Database password (default: `testpass`)
- `WORKLOAD_ITERATIONS`: Number of workload iterations (default: `5`)
- `WORKLOAD_DELAY`: Delay between iterations in seconds (default: `2`)
- `WORKLOAD_TYPE`: Type of workload to run (default: `basic`)

### Workload Types

#### `basic`
- Runs the standard minimal_workload.sql
- Creates demo tables and runs basic queries
- Good for initial testing and demonstration

#### `intensive`
- Generates large amounts of test data
- Runs complex analytical queries
- Includes slow queries and temporary tables
- Best for performance analysis

#### `errors`
- Intentionally generates various types of errors
- Tests error handling and logging
- Useful for debugging scenarios

#### `mixed`
- Alternates between basic, intensive, and error workloads
- Provides comprehensive test coverage
- Simulates real-world mixed workloads

## Usage Examples

### Basic Usage
```bash
# Start the environment
docker-compose up -d

# Check logs
docker-compose logs postgres
docker-compose logs workload

# Connect to PostgreSQL
docker-compose exec postgres psql -U testuser -d testdb
```

### Custom Workload Examples
```bash
# Run intensive workload with 10 iterations
docker-compose run --rm \
  -e WORKLOAD_TYPE=intensive \
  -e WORKLOAD_ITERATIONS=10 \
  -e WORKLOAD_DELAY=1 \
  workload

# Run mixed workload for extended testing
docker-compose run --rm \
  -e WORKLOAD_TYPE=mixed \
  -e WORKLOAD_ITERATIONS=20 \
  -e WORKLOAD_DELAY=3 \
  workload

# Run error workload for testing error handling
docker-compose run --rm \
  -e WORKLOAD_TYPE=errors \
  -e WORKLOAD_ITERATIONS=5 \
  workload
```

### Using with pg-loggrep

1. **Start the environment and generate logs**:
   ```bash
   docker-compose up -d postgres
   docker-compose run --rm -e WORKLOAD_TYPE=mixed -e WORKLOAD_ITERATIONS=10 workload
   ```

2. **Access the log files**:
   ```bash
   # Logs are stored in the postgres_logs Docker volume
   # Copy logs to local directory for analysis
   docker run --rm -v pg-loggrep_postgres_logs:/logs -v $(pwd):/output alpine \
     cp -r /logs /output/
   ```

3. **Analyze with pg-loggrep**:
   ```bash
   # Assuming you've copied logs to ./logs/
   cargo run -- --input ./logs/postgresql-*.log --output analysis.json --extension json
   ```

## Log Configuration

The PostgreSQL instance is configured with comprehensive logging:

- **All statements logged**: `log_statement = 'all'`
- **Duration logging**: `log_duration = on`
- **Minimum duration**: `log_min_duration_statement = 0`
- **Connection logging**: `log_connections = on`, `log_disconnections = on`
- **Detailed line prefix**: `%m [%p] %q%u@%d %a: `
- **Lock wait logging**: `log_lock_waits = on`
- **Checkpoint logging**: `log_checkpoints = on`
- **Temporary file logging**: `log_temp_files = 0`

## File Structure

```
demo/docker/
├── docker-compose.yml          # Main compose configuration
├── README.md                   # This file
├── postgres/
│   ├── Dockerfile             # PostgreSQL 17 container
│   └── postgresql.conf        # Comprehensive logging configuration
└── workload/
    ├── Dockerfile             # Workload generator container
    └── minimal_workload.sql   # Base SQL workload
```

## Volumes

- `postgres_data`: Persistent PostgreSQL data
- `postgres_logs`: Shared log directory accessible by both services

## Networking

- `pg-loggrep-network`: Bridge network for service communication
- PostgreSQL port 5432 exposed to host for external connections

## Health Checks

- PostgreSQL service includes health check using `pg_isready`
- Workload service waits for PostgreSQL to be healthy before starting
- 30-second startup period with 10-second intervals

## Troubleshooting

### PostgreSQL Won't Start
```bash
# Check logs
docker-compose logs postgres

# Verify configuration
docker-compose exec postgres postgres --config-file=/etc/postgresql/postgresql.conf --check

# Reset data volume if needed
docker-compose down -v
docker-compose up postgres
```

### Workload Connection Issues
```bash
# Test connectivity
docker-compose exec workload pg_isready -h postgres -p 5432 -U testuser

# Check environment variables
docker-compose run --rm workload env | grep POSTGRES
```

### Log Access Issues
```bash
# Check log volume
docker volume inspect pg-loggrep_postgres_logs

# List log files
docker run --rm -v pg-loggrep_postgres_logs:/logs alpine ls -la /logs
```

### Performance Issues
```bash
# Monitor resource usage
docker stats

# Check PostgreSQL performance
docker-compose exec postgres psql -U testuser -d testdb -c "SELECT * FROM pg_stat_activity;"
```

## Development

### Rebuilding Images
```bash
# Rebuild all images
docker-compose build

# Rebuild specific service
docker-compose build postgres
docker-compose build workload
```

### Customizing Configuration
1. Modify `postgres/postgresql.conf` for PostgreSQL settings
2. Modify `workload/minimal_workload.sql` for base workload
3. Rebuild and restart services

### Adding Extensions
1. Update `postgres/Dockerfile` to install additional extensions
2. Update `postgres/postgresql.conf` to load extensions
3. Rebuild PostgreSQL service

## Integration with CI/CD

This environment can be used in automated testing:

```yaml
# Example GitHub Actions step
- name: Test pg-loggrep with Docker
  run: |
    cd demo/docker
    docker-compose up -d postgres
    docker-compose run --rm -e WORKLOAD_ITERATIONS=5 workload
    # Copy logs and run pg-loggrep tests
    docker run --rm -v pg-loggrep_postgres_logs:/logs -v $(pwd):/output alpine cp -r /logs /output/
    cargo test
```

## Security Notes

- Default credentials are for development only
- Change passwords for production use
- Consider using Docker secrets for sensitive data
- Network is isolated by default but PostgreSQL port is exposed

## Contributing

When modifying this environment:

1. Test all workload types
2. Verify log generation and accessibility
3. Update documentation for any new features
4. Ensure backward compatibility with existing scripts
