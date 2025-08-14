#!/bin/bash

# Run demo script for pg-loggrep

set -e

echo "Starting pg-loggrep demo environment..."

# Change to docker directory
cd demo/docker

# Start the services
echo "Starting PostgreSQL and workload services..."
docker-compose up -d

# Wait for PostgreSQL to be ready
echo "Waiting for PostgreSQL to be ready..."
sleep 10

# Check if PostgreSQL is running
if docker-compose ps | grep -q "Up"; then
    echo "Demo environment is running!"
    echo ""
    echo "PostgreSQL is available at:"
    echo "  Host: localhost"
    echo "  Port: 5432"
    echo "  Database: testdb"
    echo "  Username: testuser"
    echo "  Password: testpass"
    echo ""
    echo "Logs will be available in: demo/logs/"
    echo ""
    echo "To generate workload, run: ./scripts/run-workload.sh"
    echo "To stop the demo, run: ./scripts/cleanup.sh"
else
    echo "Error: Failed to start demo environment"
    docker-compose logs
    exit 1
fi
