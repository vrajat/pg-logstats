#!/bin/bash

# Docker Demo Script for pg-loggrep
# This script helps users quickly set up and test the Docker environment

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DOCKER_DIR="$SCRIPT_DIR/../docker"
PROJECT_ROOT="$SCRIPT_DIR/../.."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Function to check if Docker is running
check_docker() {
    if ! docker info >/dev/null 2>&1; then
        print_error "Docker is not running. Please start Docker and try again."
        exit 1
    fi
    print_success "Docker is running"
}

# Function to check if docker-compose is available
check_docker_compose() {
    if ! command -v docker-compose >/dev/null 2>&1; then
        print_error "docker-compose is not installed. Please install docker-compose and try again."
        exit 1
    fi
    print_success "docker-compose is available"
}

# Function to build and start the environment
start_environment() {
    print_status "Starting PostgreSQL Docker environment..."
    cd "$DOCKER_DIR"

    # Build images
    print_status "Building Docker images..."
    docker-compose build

    # Start PostgreSQL
    print_status "Starting PostgreSQL service..."
    docker-compose up -d postgres

    # Wait for PostgreSQL to be ready
    print_status "Waiting for PostgreSQL to be ready..."
    timeout=60
    counter=0
    while ! docker-compose exec -T postgres pg_isready -U testuser -d testdb >/dev/null 2>&1; do
        if [ $counter -ge $timeout ]; then
            print_error "PostgreSQL failed to start within $timeout seconds"
            docker-compose logs postgres
            exit 1
        fi
        sleep 2
        counter=$((counter + 2))
        echo -n "."
    done
    echo
    print_success "PostgreSQL is ready!"
}

# Function to run workload
run_workload() {
    local workload_type=${1:-basic}
    local iterations=${2:-5}
    local delay=${3:-2}

    print_status "Running $workload_type workload ($iterations iterations, ${delay}s delay)..."
    cd "$DOCKER_DIR"

    docker-compose run --rm \
        -e WORKLOAD_TYPE="$workload_type" \
        -e WORKLOAD_ITERATIONS="$iterations" \
        -e WORKLOAD_DELAY="$delay" \
        workload

    print_success "Workload completed!"
}

# Function to extract logs
extract_logs() {
    local output_dir=${1:-./logs}

    print_status "Extracting PostgreSQL logs to $output_dir..."

    # Create output directory
    mkdir -p "$output_dir"

    # Extract logs from Docker volume
    docker run --rm \
        -v docker_postgres_logs:/logs:ro \
        -v "$(realpath "$output_dir"):/output" \
        alpine \
        sh -c "cp -r /logs/* /output/ 2>/dev/null || echo 'No log files found'"

    # Check if logs were extracted
    if [ "$(ls -A "$output_dir" 2>/dev/null)" ]; then
        print_success "Logs extracted to $output_dir"
        ls -la "$output_dir"
    else
        print_warning "No log files found. Make sure PostgreSQL has been running and generating logs."
    fi
}

# Function to analyze logs with pg-loggrep
analyze_logs() {
    local log_dir=${1:-./logs}
    local output_file=${2:-analysis.json}

    print_status "Analyzing logs with pg-loggrep..."

    if [ ! -d "$log_dir" ] || [ -z "$(ls -A "$log_dir" 2>/dev/null)" ]; then
        print_error "Log directory $log_dir is empty or doesn't exist. Run extract_logs first."
        return 1
    fi

    cd "$PROJECT_ROOT"

    # Build pg-loggrep if needed
    if [ ! -f "target/release/pg-loggrep" ] && [ ! -f "target/debug/pg-loggrep" ]; then
        print_status "Building pg-loggrep..."
        cargo build --release
    fi

    # Run analysis
    print_status "Running pg-loggrep analysis..."
    cargo run --release -- \
        --input "$log_dir"/postgresql-*.log \
        --output "$output_file" \
        --extension json

    print_success "Analysis completed! Results saved to $output_file"
}

# Function to stop and clean up
cleanup() {
    print_status "Stopping and cleaning up Docker environment..."
    cd "$DOCKER_DIR"

    docker-compose down
    print_success "Environment stopped"

    if [ "$1" = "--volumes" ]; then
        print_warning "Removing all volumes (this will delete all data and logs)..."
        docker-compose down -v
        print_success "Volumes removed"
    fi
}

# Function to show logs
show_logs() {
    local service=${1:-postgres}
    cd "$DOCKER_DIR"

    print_status "Showing logs for $service service..."
    docker-compose logs -f "$service"
}

# Function to connect to PostgreSQL
connect_db() {
    cd "$DOCKER_DIR"

    print_status "Connecting to PostgreSQL..."
    print_status "Database: testdb, User: testuser, Password: testpass"
    docker-compose exec postgres psql -U testuser -d testdb
}

# Function to show help
show_help() {
    cat << EOF
PostgreSQL Docker Demo Script for pg-loggrep

Usage: $0 <command> [options]

Commands:
    start                           Start PostgreSQL environment
    workload [type] [iter] [delay]  Run workload (type: basic|intensive|errors|mixed)
    extract [output_dir]            Extract logs from Docker volume
    analyze [log_dir] [output_file] Analyze logs with pg-loggrep
    full-demo                       Run complete demo (start + workload + extract + analyze)
    logs [service]                  Show service logs (postgres|workload)
    connect                         Connect to PostgreSQL database
    stop                            Stop environment
    cleanup [--volumes]             Stop and optionally remove volumes
    help                            Show this help

Examples:
    $0 start                                    # Start PostgreSQL
    $0 workload intensive 10 1                 # Run intensive workload, 10 iterations, 1s delay
    $0 extract ./my-logs                       # Extract logs to ./my-logs
    $0 analyze ./my-logs results.json          # Analyze logs and save to results.json
    $0 full-demo                               # Run complete demonstration
    $0 cleanup --volumes                       # Clean up everything including data

Environment Variables:
    WORKLOAD_TYPE       Type of workload (basic|intensive|errors|mixed)
    WORKLOAD_ITERATIONS Number of iterations
    WORKLOAD_DELAY      Delay between iterations in seconds
EOF
}

# Function to run full demo
full_demo() {
    print_status "Running full pg-loggrep Docker demonstration..."

    # Start environment
    start_environment

    # Run mixed workload
    run_workload "mixed" 10 2

    # Extract logs
    extract_logs "./demo-logs"

    # Analyze logs
    analyze_logs "./demo-logs" "demo-analysis.json"

    print_success "Full demonstration completed!"
    print_status "Check demo-analysis.json for results"
    print_status "Log files are in ./demo-logs/"
    print_status "Run '$0 connect' to explore the database"
    print_status "Run '$0 cleanup' when done"
}

# Main script logic
case "${1:-help}" in
    "start")
        check_docker
        check_docker_compose
        start_environment
        ;;
    "workload")
        check_docker
        run_workload "$2" "$3" "$4"
        ;;
    "extract")
        check_docker
        extract_logs "$2"
        ;;
    "analyze")
        analyze_logs "$2" "$3"
        ;;
    "full-demo")
        check_docker
        check_docker_compose
        full_demo
        ;;
    "logs")
        check_docker
        show_logs "$2"
        ;;
    "connect")
        check_docker
        connect_db
        ;;
    "stop")
        check_docker
        cd "$DOCKER_DIR"
        docker-compose stop
        print_success "Environment stopped"
        ;;
    "cleanup")
        check_docker
        cleanup "$2"
        ;;
    "help"|*)
        show_help
        ;;
esac
