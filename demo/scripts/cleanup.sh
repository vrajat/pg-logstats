#!/bin/bash

# Cleanup script for pg-loggrep demo environment

set -e

echo "Cleaning up pg-loggrep demo environment..."

# Change to docker directory
cd demo/docker

# Stop and remove containers
echo "Stopping and removing containers..."
docker-compose down

# Remove volumes (optional - uncomment if you want to remove all data)
# echo "Removing volumes..."
# docker-compose down -v

# Remove images (optional - uncomment if you want to remove images)
# echo "Removing images..."
# docker-compose down --rmi all

echo "Cleanup completed!"
echo ""
echo "Demo environment has been stopped and cleaned up."
echo "To start again, run: ./scripts/run-demo.sh"
