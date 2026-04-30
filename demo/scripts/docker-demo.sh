#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DOCKER_DIR="$(cd "$SCRIPT_DIR/../docker" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

usage() {
    cat <<'EOF'
pg-logstats Docker demo helper

Usage:
  demo/scripts/docker-demo.sh start
  demo/scripts/docker-demo.sh workload [type] [iterations] [delay]
  demo/scripts/docker-demo.sh extract [output-dir]
  demo/scripts/docker-demo.sh analyze [log-dir] [output-file]
  demo/scripts/docker-demo.sh stop
  demo/scripts/docker-demo.sh cleanup

Commands:
  start       Start PostgreSQL.
  workload    Run the workload container. Type: basic, intensive, errors, mixed.
  extract     Copy PostgreSQL logs from the Docker volume to a host directory.
  analyze     Run top query-families on copied logs and write findings JSON.
  stop        Stop containers.
  cleanup     Stop containers and remove volumes.
EOF
}

compose() {
    if command -v docker-compose >/dev/null 2>&1; then
        docker-compose "$@"
    else
        docker compose "$@"
    fi
}

require_docker() {
    if ! docker info >/dev/null 2>&1; then
        echo "Docker is not running." >&2
        exit 1
    fi
}

start() {
    require_docker
    cd "$DOCKER_DIR"
    compose up -d postgres
}

workload() {
    require_docker
    local workload_type="${1:-basic}"
    local iterations="${2:-5}"
    local delay="${3:-2}"

    case "$workload_type" in
        basic|intensive|errors|mixed) ;;
        *)
            echo "Invalid workload type: $workload_type" >&2
            exit 2
            ;;
    esac

    cd "$DOCKER_DIR"
    compose run --rm \
        -e WORKLOAD_TYPE="$workload_type" \
        -e WORKLOAD_ITERATIONS="$iterations" \
        -e WORKLOAD_DELAY="$delay" \
        workload
}

extract_logs() {
    require_docker
    local output_dir="${1:-$PROJECT_ROOT/demo/output/docker-logs}"
    mkdir -p "$output_dir"

    docker run --rm \
        -v pg-logstats_postgres_logs:/logs:ro \
        -v "$(cd "$output_dir" && pwd):/output" \
        alpine \
        sh -c "cp -r /logs/. /output/ 2>/dev/null || true"

    if [[ -z "$(find "$output_dir" -type f -print -quit)" ]]; then
        echo "No log files were copied into $output_dir" >&2
        exit 1
    fi

    echo "Copied logs to $output_dir"
}

analyze() {
    local log_dir="${1:-$PROJECT_ROOT/demo/output/docker-logs}"
    local output_file="${2:-$PROJECT_ROOT/demo/output/docker-findings.json}"

    if [[ ! -d "$log_dir" ]]; then
        echo "Log directory does not exist: $log_dir" >&2
        exit 1
    fi

    cd "$PROJECT_ROOT"
    cargo run --quiet -- \
        --quiet \
        top query-families \
        --log-dir "$log_dir" \
        --output-format json \
        --outfile "$output_file"

    echo "Wrote $output_file"
}

stop() {
    cd "$DOCKER_DIR"
    compose down
}

cleanup() {
    cd "$DOCKER_DIR"
    compose down -v
}

command="${1:-help}"
shift || true

case "$command" in
    start)
        start "$@"
        ;;
    workload)
        workload "$@"
        ;;
    extract)
        extract_logs "$@"
        ;;
    analyze)
        analyze "$@"
        ;;
    stop)
        stop "$@"
        ;;
    cleanup)
        cleanup "$@"
        ;;
    -h|--help|help)
        usage
        ;;
    *)
        echo "Unknown command: $command" >&2
        usage
        exit 2
        ;;
esac
