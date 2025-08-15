#!/bin/bash

# Full end-to-end demo script for pg-loggrep
# This script orchestrates the complete demonstration workflow

set -e

# Script configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DOCKER_DIR="$SCRIPT_DIR/../docker"
PROJECT_ROOT="$SCRIPT_DIR/../.."

# Default configuration
VERBOSE=${VERBOSE:-false}
WORKLOAD_DURATION=${WORKLOAD_DURATION:-180}  # 3 minutes default
WORKLOAD_TYPE=${WORKLOAD_TYPE:-mixed}
OUTPUT_FORMAT=${OUTPUT_FORMAT:-json}
PRETTY_FORMAT=${PRETTY_FORMAT:-true}
AUTO_CLEANUP=${AUTO_CLEANUP:-false}

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
pg-loggrep Full End-to-End Demo Script

This script runs a complete demonstration of pg-loggrep, including:
- Setting up the PostgreSQL environment
- Running workloads to generate log data
- Extracting and analyzing logs with pg-loggrep
- Displaying results with pretty formatting
- Showing example jq queries for analysis

Usage: $0 [OPTIONS]

Options:
    -h, --help                  Show this help message
    -v, --verbose               Enable verbose output
    -d, --duration SECONDS      Workload duration in seconds (default: 180)
    -t, --type TYPE             Workload type (basic|intensive|errors|mixed)
    -f, --format FORMAT         Output format (json|text) (default: json)
    --no-pretty                 Disable pretty formatting
    --auto-cleanup              Automatically cleanup after demo
    --skip-setup                Skip environment setup
    --skip-workload             Skip workload generation
    --skip-analysis             Skip log analysis

Environment Variables:
    VERBOSE=true                Enable verbose output
    WORKLOAD_DURATION=SECONDS   Set workload duration
    WORKLOAD_TYPE=TYPE          Set workload type
    OUTPUT_FORMAT=FORMAT        Set output format
    PRETTY_FORMAT=false         Disable pretty formatting
    AUTO_CLEANUP=true           Enable automatic cleanup

Examples:
    $0                                      # Run full demo (3 min mixed workload)
    $0 --duration 300 --type intensive      # 5 min intensive workload demo
    $0 --verbose --auto-cleanup             # Verbose demo with auto cleanup
    $0 --skip-setup --duration 60           # Skip setup, run 1 min demo

Exit Codes:
    0   Success
    1   Setup failed
    2   Invalid parameters
    3   Workload failed
    4   Analysis failed
    130 Interrupted by user
EOF
}

# Function to validate parameters
validate_parameters() {
    if ! [[ "$WORKLOAD_DURATION" =~ ^[0-9]+$ ]] || [ "$WORKLOAD_DURATION" -lt 30 ]; then
        print_error "Invalid duration: $WORKLOAD_DURATION (must be at least 30 seconds)"
        exit 2
    fi

    case "$WORKLOAD_TYPE" in
        basic|intensive|errors|mixed)
            ;;
        *)
            print_error "Invalid workload type: $WORKLOAD_TYPE"
            print_error "Valid types: basic, intensive, errors, mixed"
            exit 2
            ;;
    esac

    case "$OUTPUT_FORMAT" in
        json|text)
            ;;
        *)
            print_error "Invalid output format: $OUTPUT_FORMAT"
            print_error "Valid formats: json, text"
            exit 2
            ;;
    esac
}

# Function to check if jq is available
check_jq() {
    if ! command -v jq &> /dev/null; then
        print_warning "jq is not installed - pretty formatting will be limited"
        print_warning "Install jq for better JSON formatting: https://stedolan.github.io/jq/"
        PRETTY_FORMAT=false
    else
        print_verbose "jq is available for JSON formatting"
    fi
}

# Function to run setup
run_setup() {
    if [ "$SKIP_SETUP" = "true" ]; then
        print_status "Skipping environment setup"
        return 0
    fi

    print_status "Setting up demo environment..."

    if [ "$VERBOSE" = "true" ]; then
        "$SCRIPT_DIR/setup.sh" --verbose
    else
        "$SCRIPT_DIR/setup.sh"
    fi

    if [ $? -ne 0 ]; then
        print_error "Setup failed"
        exit 1
    fi

    print_success "Environment setup completed"
}

# Function to start services
start_services() {
    print_status "Starting PostgreSQL services..."
    cd "$DOCKER_DIR"

    if [ "$VERBOSE" = "true" ]; then
        docker-compose up -d
    else
        docker-compose up -d >/dev/null 2>&1
    fi

    # Wait for PostgreSQL to be ready
    print_status "Waiting for PostgreSQL to be ready..."
    local max_attempts=60
    local attempt=1

    while [ $attempt -le $max_attempts ]; do
        if docker-compose exec -T postgres pg_isready -U testuser -d testdb >/dev/null 2>&1; then
            print_success "PostgreSQL is ready"
            break
        fi

        if [ $attempt -eq $max_attempts ]; then
            print_error "PostgreSQL failed to start within $max_attempts attempts"
            docker-compose logs postgres
            exit 1
        fi

        print_verbose "Waiting for PostgreSQL... (attempt $attempt/$max_attempts)"
        sleep 2
        ((attempt++))
    done
}

# Function to run workload
run_workload() {
    if [ "$SKIP_WORKLOAD" = "true" ]; then
        print_status "Skipping workload generation"
        return 0
    fi

    print_status "Running workload to generate log data..."
    print_status "Workload: $WORKLOAD_TYPE, Duration: ${WORKLOAD_DURATION}s"

    local workload_args="--type $WORKLOAD_TYPE --duration $WORKLOAD_DURATION"
    if [ "$VERBOSE" = "true" ]; then
        workload_args="$workload_args --verbose"
    fi

    "$SCRIPT_DIR/run-workload.sh" $workload_args

    if [ $? -ne 0 ]; then
        print_error "Workload execution failed"
        exit 3
    fi

    print_success "Workload generation completed"
}

# Function to extract logs
extract_logs() {
    print_status "Extracting PostgreSQL logs..."

    local log_dir="$PROJECT_ROOT/demo/output/logs"
    mkdir -p "$log_dir"

    # Use docker-demo.sh to extract logs
    "$SCRIPT_DIR/docker-demo.sh" extract "$log_dir"

    if [ $? -ne 0 ]; then
        print_error "Log extraction failed"
        exit 4
    fi

    # Check if logs were extracted
    if [ -z "$(ls -A "$log_dir" 2>/dev/null)" ]; then
        print_warning "No log files found in $log_dir"
        print_warning "Trying alternative extraction method..."

        # Alternative: get logs from docker-compose logs
        cd "$DOCKER_DIR"
        docker-compose logs postgres > "$log_dir/postgres-container.log" 2>&1

        if [ -s "$log_dir/postgres-container.log" ]; then
            print_success "Extracted logs from container output"
        else
            print_error "Failed to extract any logs"
            exit 4
        fi
    else
        print_success "Logs extracted to $log_dir"
        print_verbose "Log files: $(ls -la "$log_dir")"
    fi
}

# Function to analyze logs
analyze_logs() {
    if [ "$SKIP_ANALYSIS" = "true" ]; then
        print_status "Skipping log analysis"
        return 0
    fi

    print_status "Analyzing logs with pg-loggrep..."

    local log_dir="$PROJECT_ROOT/demo/output/logs"
    local output_file="$PROJECT_ROOT/demo/output/analysis.$OUTPUT_FORMAT"

    # Build pg-loggrep if needed
    cd "$PROJECT_ROOT"
    if [ ! -f "target/release/pg-loggrep" ] && [ ! -f "target/debug/pg-loggrep" ]; then
        print_status "Building pg-loggrep..."
        if [ "$VERBOSE" = "true" ]; then
            cargo build --release
        else
            cargo build --release >/dev/null 2>&1
        fi
    fi

    # Run analysis
    print_status "Running pg-loggrep analysis..."
    local log_files=$(find "$log_dir" -name "*.log" -type f)

    if [ -z "$log_files" ]; then
        print_error "No log files found for analysis"
        exit 4
    fi

    local analysis_args=""
    if [ "$OUTPUT_FORMAT" = "json" ]; then
        analysis_args="--output $output_file --extension json"
    else
        analysis_args="--output $output_file"
    fi

    if [ "$VERBOSE" = "true" ]; then
        cargo run --release -- $log_files $analysis_args
    else
        cargo run --release -- $log_files $analysis_args >/dev/null 2>&1
    fi

    if [ $? -ne 0 ]; then
        print_error "Log analysis failed"
        exit 4
    fi

    print_success "Analysis completed: $output_file"
}

# Function to display results with pretty formatting
display_results() {
    local output_file="$PROJECT_ROOT/demo/output/analysis.$OUTPUT_FORMAT"

    if [ ! -f "$output_file" ]; then
        print_error "Analysis file not found: $output_file"
        return 1
    fi

    print_status "Displaying analysis results..."
    echo ""
    echo "=========================================="
    echo "         pg-loggrep Analysis Results"
    echo "=========================================="
    echo ""

    if [ "$OUTPUT_FORMAT" = "json" ] && [ "$PRETTY_FORMAT" = "true" ] && command -v jq &> /dev/null; then
        # Pretty print JSON with jq
        echo "Summary Statistics:"
        jq -r '
            if .summary then
                "  Total Entries: " + (.summary.total_entries // 0 | tostring) + "\n" +
                "  Error Count: " + (.summary.error_count // 0 | tostring) + "\n" +
                "  Warning Count: " + (.summary.warning_count // 0 | tostring) + "\n" +
                "  Query Count: " + (.summary.query_count // 0 | tostring)
            else
                "  Summary not available"
            end
        ' "$output_file"

        echo ""
        echo "Recent Entries (last 5):"
        jq -r '
            if .entries then
                .entries[-5:] | .[] |
                "  [" + .timestamp + "] " + .level + ": " + (.message // .query // "No message")
            else
                "  No entries found"
            end
        ' "$output_file" | head -20

    else
        # Simple display for text format or when jq is not available
        if [ "$OUTPUT_FORMAT" = "json" ]; then
            echo "Analysis Results (JSON format):"
            head -50 "$output_file"
        else
            echo "Analysis Results:"
            head -50 "$output_file"
        fi
    fi

    echo ""
    echo "Full results available in: $output_file"
}

# Function to show example jq queries
show_jq_examples() {
    if [ "$OUTPUT_FORMAT" != "json" ] || ! command -v jq &> /dev/null; then
        return 0
    fi

    local output_file="$PROJECT_ROOT/demo/output/analysis.$OUTPUT_FORMAT"

    echo ""
    echo "=========================================="
    echo "         Example jq Queries"
    echo "=========================================="
    echo ""

    cat << 'EOF'
Common analysis queries you can run:

1. Count entries by log level:
   jq '.entries | group_by(.level) | map({level: .[0].level, count: length})' analysis.json

2. Find all ERROR entries:
   jq '.entries[] | select(.level == "ERROR")' analysis.json

3. Get slow queries (if duration > 1000ms):
   jq '.entries[] | select(.duration and (.duration | tonumber) > 1000)' analysis.json

4. Count queries by type:
   jq '.entries | map(select(.query)) | group_by(.query | split(" ")[0]) | map({type: .[0].query | split(" ")[0], count: length})' analysis.json

5. Timeline of events (last 10):
   jq '.entries[-10:] | .[] | .timestamp + " " + .level + ": " + (.message // .query // "")' analysis.json

6. Find connection events:
   jq '.entries[] | select(.message and (.message | contains("connection")))' analysis.json

EOF

    echo "Try these queries with your analysis file:"
    echo "  $output_file"
    echo ""
}

# Function to cleanup
cleanup_demo() {
    if [ "$AUTO_CLEANUP" = "true" ]; then
        print_status "Performing automatic cleanup..."
        "$SCRIPT_DIR/cleanup.sh"
    else
        echo ""
        echo "Demo completed! To clean up the environment, run:"
        echo "  ./scripts/cleanup.sh"
    fi
}

# Function to handle interrupts
handle_interrupt() {
    print_warning "Demo interrupted by user"
    cleanup_demo
    exit 130
}

# Parse command line arguments
SKIP_SETUP=false
SKIP_WORKLOAD=false
SKIP_ANALYSIS=false

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
        -d|--duration)
            WORKLOAD_DURATION="$2"
            shift 2
            ;;
        -t|--type)
            WORKLOAD_TYPE="$2"
            shift 2
            ;;
        -f|--format)
            OUTPUT_FORMAT="$2"
            shift 2
            ;;
        --no-pretty)
            PRETTY_FORMAT=false
            shift
            ;;
        --auto-cleanup)
            AUTO_CLEANUP=true
            shift
            ;;
        --skip-setup)
            SKIP_SETUP=true
            shift
            ;;
        --skip-workload)
            SKIP_WORKLOAD=true
            shift
            ;;
        --skip-analysis)
            SKIP_ANALYSIS=true
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
    print_status "Starting pg-loggrep full end-to-end demonstration..."
    print_status "Configuration: ${WORKLOAD_TYPE} workload, ${WORKLOAD_DURATION}s duration, ${OUTPUT_FORMAT} output"

    # Validate parameters
    validate_parameters
    check_jq

    # Run demo steps
    run_setup
    start_services
    run_workload
    extract_logs
    analyze_logs
    display_results
    show_jq_examples
    cleanup_demo

    print_success "Full demonstration completed successfully!"
    echo ""
    echo "Check the demo/output/ directory for all generated files."
}

# Handle interrupts gracefully
trap handle_interrupt INT TERM

# Run main function
main "$@"
