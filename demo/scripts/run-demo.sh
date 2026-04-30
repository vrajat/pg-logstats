#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
OUTPUT_DIR="$PROJECT_ROOT/demo/output"
FINDINGS_FILE="$OUTPUT_DIR/findings.json"
DIFF_FILE="$OUTPUT_DIR/diff-findings.json"

usage() {
    cat <<'EOF'
pg-logstats fixture demo

Usage:
  demo/scripts/run-demo.sh [--no-build]

Runs the supported CLI workflows against checked-in fixture logs:
  - top query-families
  - slow-queries diff
  - suggest-sql

Output files:
  demo/output/findings.json
  demo/output/diff-findings.json
EOF
}

NO_BUILD=false
while [[ $# -gt 0 ]]; do
    case "$1" in
        --no-build)
            NO_BUILD=true
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            usage
            exit 2
            ;;
    esac
done

cd "$PROJECT_ROOT"
mkdir -p "$OUTPUT_DIR"

if [[ "$NO_BUILD" != "true" ]]; then
    cargo build
fi

echo "== top query-families =="
cargo run --quiet -- top query-families demo/logs/sample_stderr.log

echo
echo "== write findings JSON =="
cargo run --quiet -- \
    --quiet \
    top query-families \
    --output-format json \
    --outfile "$FINDINGS_FILE" \
    demo/logs/sample_stderr.log
echo "Wrote $FINDINGS_FILE"

echo
echo "== slow-queries diff =="
cargo run --quiet -- \
    --quiet \
    slow-queries diff \
    --baseline demo/logs/diff_baseline.log \
    --target demo/logs/diff_target.log

echo
echo "== write diff JSON =="
cargo run --quiet -- \
    --quiet \
    slow-queries diff \
    --output-format json \
    --outfile "$DIFF_FILE" \
    --baseline demo/logs/diff_baseline.log \
    --target demo/logs/diff_target.log
echo "Wrote $DIFF_FILE"

echo
echo "== suggest-sql =="
cargo run --quiet -- \
    --quiet \
    suggest-sql \
    --findings-file "$FINDINGS_FILE" \
    --rank 1
