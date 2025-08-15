#!/bin/bash

# Setup script for pg-loggrep demo environment
# This script checks for required tools, builds containers, and prepares the environment

set -e

# Script configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DOCKER_DIR="$SCRIPT_DIR/../docker"
PROJECT_ROOT="$SCRIPT_DIR/../.."

# Default configuration
VERBOSE=${VERBOSE:-false}
FORCE_REBUILD=${FORCE_REBUILD:-false}

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output with timestamps
print_status() {
    echo -e "$(date '+%Y-%m-%d %H:%M:%S') ${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "$(date '+%Y-%m-%d %H:%M:%S') ${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "$(date '+%Y-%m-%d %H:%M:%S') ${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "$(date '+%Y-%m-%d %H:%M:%S') ${RED}[ERROR]${NC} $1"
}

print_verbose() {
    if [ "$VERBOSE" = "true" ]; then
        echo -e "$(date '+%Y-%m-%d %H:%M:%S') ${BLUE}[VERBOSE]${NC} $1"
    fi
}

# Function to show help
show_help() {
    cat << EOF
pg-loggrep Demo Setup Script

This script sets up the complete demo environment for pg-loggrep, including:
- Checking for required tools (Docker, Docker Compose)
- Building PostgreSQL and workload containers
- Creating necessary directories and permissions
- Verifying the environment is ready

Usage: $0 [OPTIONS]

Options:
    -h, --help          Show this help message
    -v, --verbose       Enable verbose output
    -f, --force         Force rebuild of Docker images
    --no-build          Skip Docker image building

Environment Variables:
    VERBOSE=true        Enable verbose output
    FORCE_REBUILD=true  Force rebuild of Docker images

Examples:
    $0                  # Standard setup
    $0 --verbose        # Setup with detailed output
    $0 --force          # Force rebuild all images
    VERBOSE=true $0     # Setup with verbose output via env var

Exit Codes:
    0   Success
    1   Docker not available
    2   Docker Compose not available
    3   Build failed
    4   Environment verification failed
EOF
}

# Function to check if Docker is available and running
check_docker() {
    print_status "Checking Docker availability..."

    if ! command -v docker &> /dev/null; then
        print_error "Docker is not installed or not in PATH"
        print_error "Please install Docker from https://docs.docker.com/get-docker/"
        exit 1
    fi

    if ! docker info >/dev/null 2>&1; then
        print_error "Docker daemon is not running"
        print_error "Please start Docker and try again"
        exit 1
    fi

    print_success "Docker is available and running"
    print_verbose "Docker version: $(docker --version)"
}

# Function to check if Docker Compose is available
check_docker_compose() {
    print_status "Checking Docker Compose availability..."

    if ! command -v docker-compose &> /dev/null; then
        print_error "Docker Compose is not installed or not in PATH"
        print_error "Please install Docker Compose from https://docs.docker.com/compose/install/"
        exit 2
    fi

    print_success "Docker Compose is available"
    print_verbose "Docker Compose version: $(docker-compose --version)"
}

# Function to create necessary directories
create_directories() {
    print_status "Creating necessary directories..."

    local dirs=(
        "$PROJECT_ROOT/demo/logs"
        "$PROJECT_ROOT/demo/data"
        "$PROJECT_ROOT/demo/output"
    )

    for dir in "${dirs[@]}"; do
        if [ ! -d "$dir" ]; then
            mkdir -p "$dir"
            print_verbose "Created directory: $dir"
        else
            print_verbose "Directory already exists: $dir"
        fi
    done

    print_success "Directories created successfully"
}

# Function to set proper permissions
set_permissions() {
    print_status "Setting proper permissions..."

    # Make all scripts executable
    chmod +x "$SCRIPT_DIR"/*.sh
    print_verbose "Made scripts executable"

    # Set proper permissions for log and data directories
    chmod 755 "$PROJECT_ROOT/demo/logs" "$PROJECT_ROOT/demo/data" "$PROJECT_ROOT/demo/output" 2>/dev/null || true
    print_verbose "Set directory permissions"

    print_success "Permissions set successfully"
}

# Function to build Docker images
build_images() {
    if [ "$NO_BUILD" = "true" ]; then
        print_status "Skipping Docker image build (--no-build specified)"
        return 0
    fi

    print_status "Building Docker images..."
    cd "$DOCKER_DIR"

    local build_args=""
    if [ "$FORCE_REBUILD" = "true" ]; then
        build_args="--no-cache"
        print_verbose "Force rebuild enabled"
    fi

    if [ "$VERBOSE" = "true" ]; then
        docker-compose build $build_args
    else
        docker-compose build $build_args >/dev/null 2>&1
    fi

    if [ $? -eq 0 ]; then
        print_success "Docker images built successfully"
    else
        print_error "Failed to build Docker images"
        exit 3
    fi
}

# Function to verify the environment
verify_environment() {
    print_status "Verifying environment setup..."
    cd "$DOCKER_DIR"

    # Check if images exist
    local images=("docker-postgres" "docker-workload")
    for image in "${images[@]}"; do
        if docker images --format "table {{.Repository}}" | grep -q "^$image$"; then
            print_verbose "Image $image exists"
        else
            print_error "Image $image not found"
            exit 4
        fi
    done

    # Test docker-compose configuration
    if docker-compose config >/dev/null 2>&1; then
        print_verbose "Docker Compose configuration is valid"
    else
        print_error "Docker Compose configuration is invalid"
        exit 4
    fi

    print_success "Environment verification completed"
}

# Function to show next steps
show_next_steps() {
    print_success "Setup completed successfully!"
    echo ""
    echo "Next steps:"
    echo "1. Start the demo environment:"
    echo "   ./scripts/run-demo.sh"
    echo ""
    echo "2. Generate workload data:"
    echo "   ./scripts/run-workload.sh"
    echo ""
    echo "3. Analyze logs with pg-loggrep:"
    echo "   cargo run -- demo/logs/*.log"
    echo ""
    echo "4. Clean up when done:"
    echo "   ./scripts/cleanup.sh"
    echo ""
    echo "For a complete automated demo, run:"
    echo "   ./scripts/docker-demo.sh full-demo"
}

# Parse command line arguments
NO_BUILD=false
while [[ $# -gt 0 ]]; do
    case $1 in
        -h|--help)
            show_help
            exit 0
            ;;
        -v|--verbose)
            VERBOSE=true
            shift
            ;;
        -f|--force)
            FORCE_REBUILD=true
            shift
            ;;
        --no-build)
            NO_BUILD=true
            shift
            ;;
        *)
            print_error "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Main execution
main() {
    print_status "Starting pg-loggrep demo environment setup..."

    # Run setup steps
    check_docker
    check_docker_compose
    create_directories
    set_permissions
    build_images
    verify_environment
    show_next_steps

    print_success "Setup process completed successfully!"
}

# Handle interrupts gracefully
trap 'print_error "Setup interrupted by user"; exit 130' INT TERM

# Run main function
main "$@"
