FROM alpine:3.18

# Install PostgreSQL client and bash
RUN apk add --no-cache \
    postgresql-client \
    bash \
    curl \
    && rm -rf /var/cache/apk/*

# Set working directory
WORKDIR /app

# Copy workload files
COPY minimal_workload.sql /app/
COPY run_workload.sh /app/

# Create an enhanced workload runner script
RUN cat > /app/run_workload.sh << 'EOF'
#!/bin/bash
set -e

# Configuration from environment variables
POSTGRES_HOST=${POSTGRES_HOST:-postgres}
POSTGRES_PORT=${POSTGRES_PORT:-5432}
POSTGRES_DB=${POSTGRES_DB:-testdb}
POSTGRES_USER=${POSTGRES_USER:-testuser}
POSTGRES_PASSWORD=${POSTGRES_PASSWORD:-testpass}
WORKLOAD_ITERATIONS=${WORKLOAD_ITERATIONS:-3}
WORKLOAD_DELAY=${WORKLOAD_DELAY:-1}
WORKLOAD_TYPE=${WORKLOAD_TYPE:-basic}

echo "Starting PostgreSQL workload generator..."
echo "Host: $POSTGRES_HOST:$POSTGRES_PORT"
echo "Database: $POSTGRES_DB"
echo "User: $POSTGRES_USER"
echo "Iterations: $WORKLOAD_ITERATIONS"
echo "Delay between iterations: ${WORKLOAD_DELAY}s"
echo "Workload type: $WORKLOAD_TYPE"
echo "----------------------------------------"

# Wait for PostgreSQL to be ready
echo "Waiting for PostgreSQL to be ready..."
until pg_isready -h "$POSTGRES_HOST" -p "$POSTGRES_PORT" -U "$POSTGRES_USER" -d "$POSTGRES_DB"; do
    echo "PostgreSQL is not ready yet, waiting..."
    sleep 2
done
echo "PostgreSQL is ready!"

# Function to run basic workload
run_basic_workload() {
    echo "Running basic workload from minimal_workload.sql..."
    PGPASSWORD="$POSTGRES_PASSWORD" psql \
        -h "$POSTGRES_HOST" \
        -p "$POSTGRES_PORT" \
        -U "$POSTGRES_USER" \
        -d "$POSTGRES_DB" \
        -f /app/minimal_workload.sql
}

# Function to run intensive workload
run_intensive_workload() {
    echo "Running intensive workload..."
    PGPASSWORD="$POSTGRES_PASSWORD" psql \
        -h "$POSTGRES_HOST" \
        -p "$POSTGRES_PORT" \
        -U "$POSTGRES_USER" \
        -d "$POSTGRES_DB" \
        -c "
        -- Create additional test data
        INSERT INTO demo_users (username, email)
        SELECT
            'user_' || generate_series(1, 100),
            'user_' || generate_series(1, 100) || '@test.com';

        INSERT INTO demo_orders (user_id, amount, status)
        SELECT
            (random() * 100 + 1)::int,
            (random() * 1000 + 10)::decimal(10,2),
            CASE
                WHEN random() < 0.7 THEN 'completed'
                WHEN random() < 0.9 THEN 'pending'
                ELSE 'cancelled'
            END
        FROM generate_series(1, 500);

        -- Run complex analytical queries
        SELECT
            u.username,
            COUNT(o.id) as total_orders,
            SUM(CASE WHEN o.status = 'completed' THEN o.amount ELSE 0 END) as completed_amount,
            AVG(CASE WHEN o.status = 'completed' THEN o.amount END) as avg_order_value
        FROM demo_users u
        LEFT JOIN demo_orders o ON u.id = o.user_id
        GROUP BY u.id, u.username
        HAVING COUNT(o.id) > 0
        ORDER BY completed_amount DESC
        LIMIT 20;

        -- Simulate some slow queries
        SELECT pg_sleep(0.1);
        SELECT COUNT(*) FROM demo_orders WHERE amount > (SELECT AVG(amount) FROM demo_orders);

        -- Create and drop temporary tables
        CREATE TEMP TABLE temp_analysis AS
        SELECT
            DATE_TRUNC('day', created_at) as day,
            status,
            COUNT(*) as order_count,
            SUM(amount) as total_amount
        FROM demo_orders
        GROUP BY DATE_TRUNC('day', created_at), status;

        SELECT * FROM temp_analysis ORDER BY day, status;
        DROP TABLE temp_analysis;
        "
}

# Function to run error-generating workload
run_error_workload() {
    echo "Running error-generating workload..."
    PGPASSWORD="$POSTGRES_PASSWORD" psql \
        -h "$POSTGRES_HOST" \
        -p "$POSTGRES_PORT" \
        -U "$POSTGRES_USER" \
        -d "$POSTGRES_DB" \
        -c "
        -- These queries will generate various types of errors and warnings
        SELECT * FROM non_existent_table;
        INSERT INTO demo_users (id, username) VALUES (1, 'duplicate_user');
        SELECT 1/0;
        SELECT * FROM demo_orders WHERE invalid_column = 'test';
        " || true  # Continue even if queries fail
}

# Run workload iterations
for i in $(seq 1 $WORKLOAD_ITERATIONS); do
    echo "=== Iteration $i/$WORKLOAD_ITERATIONS ==="

    case "$WORKLOAD_TYPE" in
        "basic")
            run_basic_workload
            ;;
        "intensive")
            run_intensive_workload
            ;;
        "errors")
            run_error_workload
            ;;
        "mixed")
            if [ $((i % 3)) -eq 1 ]; then
                run_basic_workload
            elif [ $((i % 3)) -eq 2 ]; then
                run_intensive_workload
            else
                run_error_workload
            fi
            ;;
        *)
            echo "Unknown workload type: $WORKLOAD_TYPE"
            run_basic_workload
            ;;
    esac

    if [ $i -lt $WORKLOAD_ITERATIONS ]; then
        echo "Waiting ${WORKLOAD_DELAY}s before next iteration..."
        sleep $WORKLOAD_DELAY
    fi
done

echo "Workload generation completed!"
echo "Check PostgreSQL logs in /var/log/postgresql/ for analysis with pg-loggrep"
EOF

# Make scripts executable
RUN chmod +x /app/run_workload.sh

# Create a simple monitoring script
RUN cat > /app/monitor_logs.sh << 'EOF'
#!/bin/bash
echo "Monitoring PostgreSQL logs..."
echo "Log files available:"
ls -la /var/log/postgresql/ 2>/dev/null || echo "Log directory not mounted or empty"
echo ""
echo "Recent log entries:"
tail -f /var/log/postgresql/*.log 2>/dev/null || echo "No log files found"
EOF

RUN chmod +x /app/monitor_logs.sh

# Default command
CMD ["/app/run_workload.sh"]
