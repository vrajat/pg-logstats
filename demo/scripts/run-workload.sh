#!/bin/bash

# Run workload script for pg-loggrep demo

set -e

echo "Running workload generator..."

# Change to docker directory
cd demo/docker

# Check if services are running
if ! docker-compose ps | grep -q "Up"; then
    echo "Error: Demo environment is not running. Please run './scripts/run-demo.sh' first."
    exit 1
fi

# Run the workload container
echo "Generating database workload..."
docker-compose run --rm workload

echo "Workload generation completed!"
echo ""
echo "PostgreSQL logs have been generated in: demo/logs/"
echo ""
echo "You can now use pg-loggrep to analyze the logs:"
echo "  cargo run -- demo/logs/*.log"
