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

# Create an enhanced workload runner script
COPY <<'EOF' /app/run_workload.sh
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
        INSERT INTO users (name, email)
        SELECT
            'intensive_user_' || generate_series(1, 100),
            'intensive_user_' || generate_series(1, 100) || '@test.com'
        ON CONFLICT (email) DO NOTHING;

        INSERT INTO products (name, category, price, stock)
        SELECT
            'intensive_product_' || generate_series(1, 50),
            CASE (generate_series % 5)
                WHEN 0 THEN 'Electronics'
                WHEN 1 THEN 'Books'
                WHEN 2 THEN 'Clothing'
                WHEN 3 THEN 'Sports'
                ELSE 'Other'
            END,
            (random() * 1000 + 10)::DECIMAL(10,2),
            (random() * 100)::INTEGER
        FROM generate_series(1, 50);

        -- Run complex analytical queries
        SELECT
            u.name,
            COUNT(o.id) as total_orders,
            SUM(CASE WHEN o.status = 'delivered' THEN o.total_amount ELSE 0 END) as completed_amount,
            AVG(CASE WHEN o.status = 'delivered' THEN o.total_amount END) as avg_order_value
        FROM users u
        LEFT JOIN orders o ON u.id = o.user_id
        GROUP BY u.id, u.name
        HAVING COUNT(o.id) > 0
        ORDER BY completed_amount DESC
        LIMIT 20;

        -- Simulate some slow queries
        SELECT pg_sleep(0.1);
        SELECT COUNT(*) FROM orders WHERE total_amount > (SELECT AVG(total_amount) FROM orders);

        -- Create and drop temporary tables
        CREATE TEMP TABLE temp_analysis AS
        SELECT
            DATE_TRUNC('day', order_date) as day,
            status,
            COUNT(*) as order_count,
            SUM(total_amount) as total_amount
        FROM orders
        GROUP BY DATE_TRUNC('day', order_date), status;

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
        INSERT INTO users (id, name, email) VALUES (1, 'duplicate_user', 'user1@example.com');
        SELECT 1/0;
        SELECT * FROM orders WHERE invalid_column = 'test';
        INSERT INTO products (name, category, price) VALUES ('test', 'test', -100);
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
echo "Check PostgreSQL logs in /var/log/postgresql/ for analysis with pg-logstats"
EOF

# Make scripts executable
RUN chmod +x /app/run_workload.sh

# Create a simple monitoring script
COPY <<'EOF' /app/monitor_logs.sh
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
