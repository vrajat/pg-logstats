#!/bin/bash

# Setup script for pg-loggrep demo environment

set -e

echo "Setting up pg-loggrep demo environment..."

# Create necessary directories
mkdir -p demo/logs
mkdir -p demo/data

# Set permissions
chmod 755 demo/scripts/*.sh

# Check if Docker and Docker Compose are available
if ! command -v docker &> /dev/null; then
    echo "Error: Docker is not installed or not in PATH"
    exit 1
fi

if ! command -v docker-compose &> /dev/null; then
    echo "Error: Docker Compose is not installed or not in PATH"
    exit 1
fi

# Build Docker images
echo "Building Docker images..."
cd demo/docker
docker-compose build

echo "Setup completed successfully!"
echo ""
echo "Next steps:"
echo "1. Run './scripts/run-demo.sh' to start the demo environment"
echo "2. Run './scripts/run-workload.sh' to generate sample data"
echo "3. Use pg-loggrep to analyze the generated logs"
