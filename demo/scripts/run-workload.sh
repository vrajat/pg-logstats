#!/bin/bash

# Run workload script for pg-loggrep demo
# This script executes the PostgreSQL workload with configurable parameters

set -e

# Script configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DOCKER_DIR="$SCRIPT_DIR/../docker"
PROJECT_ROOT="$SCRIPT_DIR/../.."

# Default configuration
VERBOSE=${VERBOSE:-false}
WORKLOAD_TYPE=${WORKLOAD_TYPE:-basic}
WORKLOAD_ITERATIONS=${WORKLOAD_ITERATIONS:-5}
WORKLOAD_DELAY=${WORKLOAD_DELAY:-2}
RUN_DURATION=${RUN_DURATION:-0}  # 0 means use iterations instead
PROGRESS_FEEDBACK=${PROGRESS_FEEDBACK:-true}

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

print_progress() {
    if [ "$PROGRESS_FEEDBACK" = "true" ]; then
        echo -e "$(date '+%Y-%m-%d %H:%M:%S') ${YELLOW}[PROGRESS]${NC} $1"
    fi
}

# Function to show help
show_help() {
    cat << EOF
pg-loggrep Workload Runner Script

This script executes PostgreSQL workloads to generate log data for pg-loggrep analysis.
It supports different workload types and provides configurable execution parameters.

Usage: $0 [OPTIONS]

Options:
    -h, --help                  Show this help message
    -v, --verbose               Enable verbose output
    -t, --type TYPE             Workload type (basic|intensive|errors|mixed)
    -i, --iterations NUM        Number of iterations (default: 5)
    -d, --delay SECONDS         Delay between iterations (default: 2)
    -D, --duration SECONDS      Run for specified duration instead of iterations
    --no-progress               Disable progress feedback
    --cleanup-on-interrupt      Clean up containers on interrupt

Environment Variables:
    VERBOSE=true                Enable verbose output
    WORKLOAD_TYPE=TYPE          Set workload type
    WORKLOAD_ITERATIONS=NUM     Set number of iterations
    WORKLOAD_DELAY=SECONDS      Set delay between iterations
    RUN_DURATION=SECONDS        Run for specified duration
    PROGRESS_FEEDBACK=false     Disable progress feedback

Workload Types:
    basic       Simple CRUD operations and queries
    intensive   Complex queries, large data operations
    errors      Generate various error conditions
    mixed       Combination of all workload types

Examples:
    $0                                      # Run basic workload (5 iterations, 2s delay)
    $0 --type intensive --iterations 10     # Run intensive workload, 10 iterations
    $0 --duration 300 --type mixed          # Run mixed workload for 5 minutes
    $0 --verbose --no-progress              # Verbose output without progress updates

Exit Codes:
    0   Success
    1   Environment not ready
    2   Invalid parameters
    3   Workload execution failed
    130 Interrupted by user
EOF
}

# Function to validate workload type
validate_workload_type() {
    case "$WORKLOAD_TYPE" in
        basic|intensive|errors|mixed)
            return 0
            ;;
        *)
            print_error "Invalid workload type: $WORKLOAD_TYPE"
            print_error "Valid types: basic, intensive, errors, mixed"
            exit 2
            ;;
    esac
}

# Function to validate numeric parameters
validate_parameters() {
    if ! [[ "$WORKLOAD_ITERATIONS" =~ ^[0-9]+$ ]] || [ "$WORKLOAD_ITERATIONS" -lt 1 ]; then
        print_error "Invalid iterations: $WORKLOAD_ITERATIONS (must be positive integer)"
        exit 2
    fi

    if ! [[ "$WORKLOAD_DELAY" =~ ^[0-9]+$ ]] || [ "$WORKLOAD_DELAY" -lt 0 ]; then
        print_error "Invalid delay: $WORKLOAD_DELAY (must be non-negative integer)"
        exit 2
    fi

    if ! [[ "$RUN_DURATION" =~ ^[0-9]+$ ]] || [ "$RUN_DURATION" -lt 0 ]; then
        print_error "Invalid duration: $RUN_DURATION (must be non-negative integer)"
        exit 2
    fi
}

# Function to check if environment is ready
check_environment() {
    print_status "Checking demo environment..."
    cd "$DOCKER_DIR"

    # Check if docker-compose.yml exists
    if [ ! -f "docker-compose.yml" ]; then
        print_error "docker-compose.yml not found in $DOCKER_DIR"
        print_error "Please run setup.sh first"
        exit 1
    fi

    # Check if PostgreSQL service is running
    if ! docker-compose ps postgres | grep -q "Up"; then
        print_error "PostgreSQL service is not running"
        print_error "Please run './scripts/run-demo.sh' first to start the environment"
        exit 1
    fi

    # Check if PostgreSQL is ready to accept connections
    print_status "Verifying PostgreSQL connectivity..."
    local max_attempts=30
    local attempt=1

    while [ $attempt -le $max_attempts ]; do
        if docker-compose exec -T postgres pg_isready -U testuser -d testdb >/dev/null 2>&1; then
            print_success "PostgreSQL is ready"
            break
        fi

        if [ $attempt -eq $max_attempts ]; then
            print_error "PostgreSQL is not responding after $max_attempts attempts"
            print_error "Check the PostgreSQL logs: docker-compose logs postgres"
            exit 1
        fi

        print_verbose "Waiting for PostgreSQL... (attempt $attempt/$max_attempts)"
        sleep 2
        ((attempt++))
    done
}

# Function to run workload with iterations
run_workload_iterations() {
    print_status "Running $WORKLOAD_TYPE workload ($WORKLOAD_ITERATIONS iterations, ${WORKLOAD_DELAY}s delay)"

    docker-compose run --rm \
        -e WORKLOAD_TYPE="$WORKLOAD_TYPE" \
        -e WORKLOAD_ITERATIONS="$WORKLOAD_ITERATIONS" \
        -e WORKLOAD_DELAY="$WORKLOAD_DELAY" \
        workload

    if [ $? -eq 0 ]; then
        print_success "Workload completed successfully"
    else
        print_error "Workload execution failed"
        exit 3
    fi
}

# Function to run workload for specified duration
run_workload_duration() {
    print_status "Running $WORKLOAD_TYPE workload for ${RUN_DURATION} seconds"

    # Calculate iterations based on duration and delay
    local calculated_iterations=$((RUN_DURATION / (WORKLOAD_DELAY + 5)))  # +5 for execution time estimate
    if [ $calculated_iterations -lt 1 ]; then
        calculated_iterations=1
    fi

    print_verbose "Calculated iterations for duration: $calculated_iterations"

    # Start workload in background
    docker-compose run --rm \
        -e WORKLOAD_TYPE="$WORKLOAD_TYPE" \
        -e WORKLOAD_ITERATIONS="$calculated_iterations" \
        -e WORKLOAD_DELAY="$WORKLOAD_DELAY" \
        workload &

    local workload_pid=$!
    local start_time=$(date +%s)
    local end_time=$((start_time + RUN_DURATION))

    # Monitor progress
    while [ $(date +%s) -lt $end_time ]; do
        if ! kill -0 $workload_pid 2>/dev/null; then
            # Workload finished early
            wait $workload_pid
            local exit_code=$?
            if [ $exit_code -eq 0 ]; then
                print_success "Workload completed successfully"
            else
                print_error "Workload execution failed"
                exit 3
            fi
            return 0
        fi

        local current_time=$(date +%s)
        local elapsed=$((current_time - start_time))
        local remaining=$((RUN_DURATION - elapsed))
        print_progress "Workload running... ${elapsed}s elapsed, ${remaining}s remaining"

        sleep 10
    done

    # Time's up, stop the workload
    print_status "Duration reached, stopping workload..."
    docker-compose stop workload >/dev/null 2>&1 || true
    wait $workload_pid 2>/dev/null || true

    print_success "Workload completed (duration-based)"
}

# Function to show workload results
show_results() {
    print_success "Workload generation completed!"
    echo ""
    echo "PostgreSQL logs have been generated and are available for analysis."
    echo ""
    echo "Log locations:"
    echo "  - Container logs: docker-compose logs postgres"
    echo "  - Extract logs: ./scripts/docker-demo.sh extract"
    echo ""
    echo "Next steps:"
    echo "1. Extract logs for analysis:"
    echo "   ./scripts/docker-demo.sh extract ./logs"
    echo ""
    echo "2. Analyze logs with pg-loggrep:"
    echo "   cargo run -- ./logs/*.log"
    echo ""
    echo "3. Or run full analysis:"
    echo "   ./scripts/docker-demo.sh analyze ./logs results.json"
}

# Function to cleanup on interrupt
cleanup_on_interrupt() {
    print_warning "Workload interrupted by user"
    if [ "$CLEANUP_ON_INTERRUPT" = "true" ]; then
        print_status "Cleaning up containers..."
        cd "$DOCKER_DIR"
        docker-compose stop workload >/dev/null 2>&1 || true
    fi
    exit 130
}

# Parse command line arguments
CLEANUP_ON_INTERRUPT=false
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
        -t|--type)
            WORKLOAD_TYPE="$2"
            shift 2
            ;;
        -i|--iterations)
            WORKLOAD_ITERATIONS="$2"
            shift 2
            ;;
        -d|--delay)
            WORKLOAD_DELAY="$2"
            shift 2
            ;;
        -D|--duration)
            RUN_DURATION="$2"
            shift 2
            ;;
        --no-progress)
            PROGRESS_FEEDBACK=false
            shift
            ;;
        --cleanup-on-interrupt)
            CLEANUP_ON_INTERRUPT=true
            shift
            ;;
        *)
            print_error "Unknown option: $1"
            echo "Use --help for usage information"
            exit 2
            ;;
    esac
done

# Main execution
main() {
    print_status "Starting pg-loggrep workload execution..."

    # Validate parameters
    validate_workload_type
    validate_parameters

    # Check environment
    check_environment

    # Run workload based on configuration
    if [ "$RUN_DURATION" -gt 0 ]; then
        run_workload_duration
    else
        run_workload_iterations
    fi

    # Show results
    show_results

    print_success "Workload execution completed successfully!"
}

# Handle interrupts gracefully
trap cleanup_on_interrupt INT TERM

# Run main function
main "$@"
