#!/bin/bash

# Batch Processing Examples for pg-loggrep
# ========================================
#
# This script demonstrates various batch processing workflows for analyzing
# PostgreSQL logs at scale using pg-loggrep.

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LOG_BASE_DIR="${LOG_BASE_DIR:-/var/log/postgresql}"
OUTPUT_DIR="${OUTPUT_DIR:-./analysis_output}"
TEMP_DIR="${TEMP_DIR:-/tmp/pg-loggrep-batch}"
PARALLEL_JOBS="${PARALLEL_JOBS:-4}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Help function
show_help() {
    cat << EOF
Batch Processing Examples for pg-loggrep

Usage: $0 [COMMAND] [OPTIONS]

Commands:
    daily-analysis      Analyze logs for a specific day
    weekly-summary      Generate weekly performance summary
    multi-day          Process multiple days of logs
    parallel-analysis  Process multiple log directories in parallel
    trend-analysis     Analyze performance trends over time
    alert-check        Check for performance alerts
    cleanup            Clean up temporary files
    help               Show this help message

Options:
    --log-dir DIR      Base directory for PostgreSQL logs (default: /var/log/postgresql)
    --output-dir DIR   Output directory for results (default: ./analysis_output)
    --date DATE        Specific date to analyze (YYYY-MM-DD format)
    --days N           Number of days to analyze (default: 7)
    --parallel N       Number of parallel jobs (default: 4)
    --format FORMAT    Output format: text, json (default: json)
    --quick            Use quick analysis mode
    --sample-size N    Sample size for large files

Examples:
    $0 daily-analysis --date 2024-01-15
    $0 weekly-summary --days 7
    $0 parallel-analysis --parallel 8
    $0 trend-analysis --days 30

Environment Variables:
    LOG_BASE_DIR       Base directory for PostgreSQL logs
    OUTPUT_DIR         Output directory for analysis results
    PARALLEL_JOBS      Number of parallel processing jobs

EOF
}

# Setup function
setup_environment() {
    log_info "Setting up batch processing environment..."

    # Create directories
    mkdir -p "$OUTPUT_DIR"
    mkdir -p "$TEMP_DIR"

    # Check if pg-loggrep is available
    if ! command -v pg-loggrep &> /dev/null; then
        log_error "pg-loggrep command not found. Please ensure it's installed and in PATH."
        exit 1
    fi

    # Check if required tools are available
    for tool in jq parallel; do
        if ! command -v "$tool" &> /dev/null; then
            log_warning "$tool not found. Some features may not work."
        fi
    done

    log_success "Environment setup complete"
}

# Daily analysis function
daily_analysis() {
    local date="$1"
    local format="${2:-json}"
    local quick_mode="${3:-false}"

    log_info "Analyzing logs for date: $date"

    local log_dir="$LOG_BASE_DIR/$date"
    local output_file="$OUTPUT_DIR/daily_analysis_${date}.${format}"

    if [[ ! -d "$log_dir" ]]; then
        log_error "Log directory not found: $log_dir"
        return 1
    fi

    local cmd="pg-loggrep --log-dir '$log_dir' --output-format '$format'"

    if [[ "$quick_mode" == "true" ]]; then
        cmd="$cmd --quick"
    fi

    log_info "Running: $cmd"

    if eval "$cmd" > "$output_file"; then
        log_success "Daily analysis complete: $output_file"

        # Generate summary if JSON format
        if [[ "$format" == "json" ]]; then
            generate_daily_summary "$output_file"
        fi
    else
        log_error "Daily analysis failed for $date"
        return 1
    fi
}

# Generate daily summary
generate_daily_summary() {
    local json_file="$1"
    local summary_file="${json_file%.json}_summary.txt"

    log_info "Generating daily summary: $summary_file"

    cat > "$summary_file" << EOF
PostgreSQL Log Analysis Summary
===============================
Date: $(basename "$json_file" | sed 's/daily_analysis_\(.*\)\.json/\1/')
Generated: $(date)

EOF

    # Extract key metrics using jq
    if command -v jq &> /dev/null; then
        {
            echo "Key Metrics:"
            echo "------------"
            jq -r '"Total Queries: " + (.summary.total_queries | tostring)' "$json_file"
            jq -r '"Average Duration: " + (.summary.avg_duration_ms | tostring) + "ms"' "$json_file"
            jq -r '"Error Count: " + (.summary.error_count | tostring)' "$json_file"
            echo ""

            echo "Query Types:"
            echo "------------"
            jq -r '.query_analysis.by_type | to_entries[] | "  " + .key + ": " + (.value | tostring)' "$json_file"
            echo ""

            echo "Top 5 Most Frequent Queries:"
            echo "----------------------------"
            jq -r '.query_analysis.most_frequent[:5][] | "  " + (.count | tostring) + "x: " + .query' "$json_file"
            echo ""

            # Performance alerts
            echo "Performance Alerts:"
            echo "------------------"
            local avg_duration
            avg_duration=$(jq -r '.summary.avg_duration_ms' "$json_file")
            if (( $(echo "$avg_duration > 100" | bc -l 2>/dev/null || echo 0) )); then
                echo "  ⚠️  High average query duration: ${avg_duration}ms"
            fi

            local error_count
            error_count=$(jq -r '.summary.error_count' "$json_file")
            if [[ "$error_count" -gt 0 ]]; then
                echo "  ⚠️  Errors detected: $error_count"
            fi

            local slow_queries
            slow_queries=$(jq -r '[.query_analysis.slowest_queries[] | select(.duration_ms > 1000)] | length' "$json_file" 2>/dev/null || echo 0)
            if [[ "$slow_queries" -gt 0 ]]; then
                echo "  ⚠️  Slow queries (>1s): $slow_queries"
            fi

            if [[ "$avg_duration" -le 100 && "$error_count" -eq 0 && "$slow_queries" -eq 0 ]]; then
                echo "  ✅ No performance issues detected"
            fi

        } >> "$summary_file"
    else
        echo "jq not available - summary generation limited" >> "$summary_file"
    fi

    log_success "Daily summary generated: $summary_file"
}

# Weekly summary function
weekly_summary() {
    local days="${1:-7}"
    local format="${2:-json}"

    log_info "Generating weekly summary for last $days days"

    local summary_file="$OUTPUT_DIR/weekly_summary_$(date +%Y%m%d).json"
    local report_file="$OUTPUT_DIR/weekly_report_$(date +%Y%m%d).md"

    # Collect daily analyses
    local daily_files=()
    for ((i=0; i<days; i++)); do
        local date
        date=$(date -d "$i days ago" +%Y-%m-%d)
        local daily_file="$OUTPUT_DIR/daily_analysis_${date}.json"

        if [[ -f "$daily_file" ]]; then
            daily_files+=("$daily_file")
        else
            log_warning "Daily analysis not found for $date, running analysis..."
            if daily_analysis "$date" "$format"; then
                daily_files+=("$daily_file")
            fi
        fi
    done

    if [[ ${#daily_files[@]} -eq 0 ]]; then
        log_error "No daily analysis files found"
        return 1
    fi

    # Combine daily analyses
    if command -v jq &> /dev/null; then
        log_info "Combining ${#daily_files[@]} daily analyses..."

        jq -s 'map({
            date: (.metadata.analysis_timestamp | split("T")[0]),
            total_queries: .summary.total_queries,
            avg_duration: .summary.avg_duration_ms,
            error_count: .summary.error_count,
            query_types: .query_analysis.by_type
        }) | sort_by(.date)' "${daily_files[@]}" > "$summary_file"

        log_success "Weekly summary generated: $summary_file"

        # Generate markdown report
        generate_weekly_report "$summary_file" "$report_file"
    else
        log_error "jq required for weekly summary generation"
        return 1
    fi
}

# Generate weekly report
generate_weekly_report() {
    local summary_file="$1"
    local report_file="$2"

    log_info "Generating weekly report: $report_file"

    cat > "$report_file" << EOF
# PostgreSQL Weekly Performance Report

**Generated:** $(date)
**Period:** $(jq -r '.[0].date' "$summary_file") to $(jq -r '.[-1].date' "$summary_file")

## Summary Statistics

EOF

    # Calculate totals and averages
    {
        echo "| Metric | Value |"
        echo "|--------|-------|"
        echo "| Total Days Analyzed | $(jq 'length' "$summary_file") |"
        echo "| Total Queries | $(jq 'map(.total_queries) | add' "$summary_file") |"
        echo "| Average Queries/Day | $(jq 'map(.total_queries) | add / length | round' "$summary_file") |"
        echo "| Average Duration | $(jq 'map(.avg_duration) | add / length | round' "$summary_file")ms |"
        echo "| Total Errors | $(jq 'map(.error_count) | add' "$summary_file") |"
        echo ""

        echo "## Daily Breakdown"
        echo ""
        echo "| Date | Queries | Avg Duration (ms) | Errors |"
        echo "|------|---------|-------------------|--------|"
        jq -r '.[] | "| " + .date + " | " + (.total_queries | tostring) + " | " + (.avg_duration | tostring) + " | " + (.error_count | tostring) + " |"' "$summary_file"
        echo ""

        echo "## Trends"
        echo ""

        # Performance trend
        local first_avg last_avg
        first_avg=$(jq -r '.[0].avg_duration' "$summary_file")
        last_avg=$(jq -r '.[-1].avg_duration' "$summary_file")

        if (( $(echo "$last_avg > $first_avg" | bc -l 2>/dev/null || echo 0) )); then
            echo "- 📈 **Performance Degradation**: Average duration increased from ${first_avg}ms to ${last_avg}ms"
        elif (( $(echo "$last_avg < $first_avg" | bc -l 2>/dev/null || echo 0) )); then
            echo "- 📉 **Performance Improvement**: Average duration decreased from ${first_avg}ms to ${last_avg}ms"
        else
            echo "- ➡️ **Stable Performance**: Average duration remained stable around ${first_avg}ms"
        fi

        # Query volume trend
        local first_queries last_queries
        first_queries=$(jq -r '.[0].total_queries' "$summary_file")
        last_queries=$(jq -r '.[-1].total_queries' "$summary_file")

        if [[ "$last_queries" -gt "$first_queries" ]]; then
            echo "- 📈 **Increased Load**: Query volume increased from $first_queries to $last_queries"
        elif [[ "$last_queries" -lt "$first_queries" ]]; then
            echo "- 📉 **Decreased Load**: Query volume decreased from $first_queries to $last_queries"
        else
            echo "- ➡️ **Stable Load**: Query volume remained stable around $first_queries queries"
        fi

        echo ""
        echo "## Recommendations"
        echo ""

        # Generate recommendations based on data
        local max_avg_duration
        max_avg_duration=$(jq 'map(.avg_duration) | max' "$summary_file")

        if (( $(echo "$max_avg_duration > 100" | bc -l 2>/dev/null || echo 0) )); then
            echo "- 🔍 **Investigate Slow Queries**: Peak average duration was ${max_avg_duration}ms"
        fi

        local total_errors
        total_errors=$(jq 'map(.error_count) | add' "$summary_file")

        if [[ "$total_errors" -gt 0 ]]; then
            echo "- 🚨 **Address Errors**: Total of $total_errors errors detected during the period"
        fi

        local avg_daily_queries
        avg_daily_queries=$(jq 'map(.total_queries) | add / length | round' "$summary_file")

        if [[ "$avg_daily_queries" -gt 10000 ]]; then
            echo "- ⚡ **Consider Connection Pooling**: High query volume ($avg_daily_queries queries/day average)"
        fi

    } >> "$report_file"

    log_success "Weekly report generated: $report_file"
}

# Multi-day analysis function
multi_day_analysis() {
    local days="${1:-7}"
    local format="${2:-json}"
    local parallel_jobs="${3:-$PARALLEL_JOBS}"

    log_info "Processing $days days of logs with $parallel_jobs parallel jobs"

    # Generate list of dates to process
    local dates=()
    for ((i=0; i<days; i++)); do
        dates+=($(date -d "$i days ago" +%Y-%m-%d))
    done

    # Create job list
    local job_file="$TEMP_DIR/jobs.txt"
    > "$job_file"

    for date in "${dates[@]}"; do
        echo "daily_analysis '$date' '$format'" >> "$job_file"
    done

    # Run parallel processing if available
    if command -v parallel &> /dev/null; then
        log_info "Running parallel analysis..."
        export -f daily_analysis generate_daily_summary log_info log_success log_error log_warning
        export OUTPUT_DIR LOG_BASE_DIR RED GREEN YELLOW BLUE NC

        parallel -j "$parallel_jobs" --colsep ' ' {1} {2} {3} :::: "$job_file"
    else
        log_warning "GNU parallel not available, processing sequentially..."
        while IFS=' ' read -r func date fmt; do
            $func "$date" "$fmt"
        done < "$job_file"
    fi

    log_success "Multi-day analysis complete"
}

# Trend analysis function
trend_analysis() {
    local days="${1:-30}"

    log_info "Performing trend analysis for $days days"

    local trend_file="$OUTPUT_DIR/trend_analysis_$(date +%Y%m%d).json"
    local chart_file="$OUTPUT_DIR/trend_chart_$(date +%Y%m%d).txt"

    # Collect existing daily analyses
    local daily_files=()
    for ((i=0; i<days; i++)); do
        local date
        date=$(date -d "$i days ago" +%Y-%m-%d)
        local daily_file="$OUTPUT_DIR/daily_analysis_${date}.json"

        if [[ -f "$daily_file" ]]; then
            daily_files+=("$daily_file")
        fi
    done

    if [[ ${#daily_files[@]} -lt 3 ]]; then
        log_error "Need at least 3 daily analyses for trend analysis. Found: ${#daily_files[@]}"
        return 1
    fi

    log_info "Analyzing trends from ${#daily_files[@]} daily analyses..."

    # Generate trend data
    jq -s 'map({
        date: (.metadata.analysis_timestamp | split("T")[0]),
        avg_duration: .summary.avg_duration_ms,
        total_queries: .summary.total_queries,
        error_count: .summary.error_count
    }) | sort_by(.date) | {
        period: {
            start: .[0].date,
            end: .[-1].date,
            days: length
        },
        metrics: {
            avg_duration: {
                values: map(.avg_duration),
                trend: ((.[-1].avg_duration - .[0].avg_duration) / .[0].avg_duration * 100),
                min: (map(.avg_duration) | min),
                max: (map(.avg_duration) | max),
                avg: (map(.avg_duration) | add / length)
            },
            query_volume: {
                values: map(.total_queries),
                trend: ((.[-1].total_queries - .[0].total_queries) / .[0].total_queries * 100),
                min: (map(.total_queries) | min),
                max: (map(.total_queries) | max),
                avg: (map(.total_queries) | add / length)
            },
            error_rate: {
                values: map(.error_count),
                total: (map(.error_count) | add),
                max_daily: (map(.error_count) | max),
                avg_daily: (map(.error_count) | add / length)
            }
        },
        daily_data: .
    }' "${daily_files[@]}" > "$trend_file"

    log_success "Trend analysis complete: $trend_file"

    # Generate simple ASCII chart
    generate_trend_chart "$trend_file" "$chart_file"
}

# Generate trend chart
generate_trend_chart() {
    local trend_file="$1"
    local chart_file="$2"

    log_info "Generating trend chart: $chart_file"

    cat > "$chart_file" << EOF
PostgreSQL Performance Trends
============================

Period: $(jq -r '.period.start' "$trend_file") to $(jq -r '.period.end' "$trend_file") ($(jq -r '.period.days' "$trend_file") days)

Average Duration Trend: $(jq -r '.metrics.avg_duration.trend | round' "$trend_file")%
Query Volume Trend: $(jq -r '.metrics.query_volume.trend | round' "$trend_file")%

Performance Summary:
- Min Avg Duration: $(jq -r '.metrics.avg_duration.min' "$trend_file")ms
- Max Avg Duration: $(jq -r '.metrics.avg_duration.max' "$trend_file")ms
- Overall Average: $(jq -r '.metrics.avg_duration.avg | round' "$trend_file")ms

Query Volume Summary:
- Min Daily Queries: $(jq -r '.metrics.query_volume.min' "$trend_file")
- Max Daily Queries: $(jq -r '.metrics.query_volume.max' "$trend_file")
- Average Daily: $(jq -r '.metrics.query_volume.avg | round' "$trend_file")

Error Summary:
- Total Errors: $(jq -r '.metrics.error_rate.total' "$trend_file")
- Max Daily Errors: $(jq -r '.metrics.error_rate.max_daily' "$trend_file")
- Average Daily: $(jq -r '.metrics.error_rate.avg_daily' "$trend_file")

Daily Data:
-----------
EOF

    # Add daily data table
    echo "Date       | Queries | Avg Duration | Errors" >> "$chart_file"
    echo "-----------|---------|--------------|-------" >> "$chart_file"
    jq -r '.daily_data[] | .date + " | " + (.total_queries | tostring) + "     | " + (.avg_duration | tostring) + "ms       | " + (.error_count | tostring)' "$trend_file" >> "$chart_file"

    log_success "Trend chart generated: $chart_file"
}

# Alert check function
alert_check() {
    local threshold_duration="${1:-100}"
    local threshold_errors="${2:-5}"

    log_info "Checking for performance alerts (duration > ${threshold_duration}ms, errors > $threshold_errors)"

    local alert_file="$OUTPUT_DIR/alerts_$(date +%Y%m%d_%H%M%S).txt"
    local alerts_found=false

    echo "PostgreSQL Performance Alerts - $(date)" > "$alert_file"
    echo "=======================================" >> "$alert_file"
    echo "" >> "$alert_file"

    # Check recent daily analyses
    for ((i=0; i<7; i++)); do
        local date
        date=$(date -d "$i days ago" +%Y-%m-%d)
        local daily_file="$OUTPUT_DIR/daily_analysis_${date}.json"

        if [[ -f "$daily_file" ]]; then
            local avg_duration error_count
            avg_duration=$(jq -r '.summary.avg_duration_ms' "$daily_file" 2>/dev/null || echo 0)
            error_count=$(jq -r '.summary.error_count' "$daily_file" 2>/dev/null || echo 0)

            if (( $(echo "$avg_duration > $threshold_duration" | bc -l 2>/dev/null || echo 0) )); then
                echo "🚨 PERFORMANCE ALERT - $date" >> "$alert_file"
                echo "   Average query duration: ${avg_duration}ms (threshold: ${threshold_duration}ms)" >> "$alert_file"
                echo "" >> "$alert_file"
                alerts_found=true
            fi

            if [[ "$error_count" -gt "$threshold_errors" ]]; then
                echo "🚨 ERROR ALERT - $date" >> "$alert_file"
                echo "   Error count: $error_count (threshold: $threshold_errors)" >> "$alert_file"
                echo "" >> "$alert_file"
                alerts_found=true
            fi
        fi
    done

    if [[ "$alerts_found" == "false" ]]; then
        echo "✅ No alerts found - system performance within normal parameters" >> "$alert_file"
        log_success "No performance alerts detected"
    else
        log_warning "Performance alerts detected - check $alert_file"
    fi

    cat "$alert_file"
}

# Cleanup function
cleanup() {
    log_info "Cleaning up temporary files..."

    if [[ -d "$TEMP_DIR" ]]; then
        rm -rf "$TEMP_DIR"
        log_success "Temporary directory cleaned: $TEMP_DIR"
    fi

    # Clean old analysis files (older than 30 days)
    find "$OUTPUT_DIR" -name "*.json" -mtime +30 -delete 2>/dev/null || true
    find "$OUTPUT_DIR" -name "*.txt" -mtime +30 -delete 2>/dev/null || true

    log_success "Cleanup complete"
}

# Main function
main() {
    local command="${1:-help}"
    shift || true

    # Parse options
    local date=""
    local days=7
    local format="json"
    local quick_mode=false
    local parallel_jobs="$PARALLEL_JOBS"

    while [[ $# -gt 0 ]]; do
        case $1 in
            --log-dir)
                LOG_BASE_DIR="$2"
                shift 2
                ;;
            --output-dir)
                OUTPUT_DIR="$2"
                shift 2
                ;;
            --date)
                date="$2"
                shift 2
                ;;
            --days)
                days="$2"
                shift 2
                ;;
            --parallel)
                parallel_jobs="$2"
                shift 2
                ;;
            --format)
                format="$2"
                shift 2
                ;;
            --quick)
                quick_mode=true
                shift
                ;;
            *)
                log_error "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done

    # Execute command
    case $command in
        daily-analysis)
            setup_environment
            if [[ -z "$date" ]]; then
                date=$(date +%Y-%m-%d)
            fi
            daily_analysis "$date" "$format" "$quick_mode"
            ;;
        weekly-summary)
            setup_environment
            weekly_summary "$days" "$format"
            ;;
        multi-day)
            setup_environment
            multi_day_analysis "$days" "$format" "$parallel_jobs"
            ;;
        parallel-analysis)
            setup_environment
            multi_day_analysis "$days" "$format" "$parallel_jobs"
            ;;
        trend-analysis)
            setup_environment
            trend_analysis "$days"
            ;;
        alert-check)
            setup_environment
            alert_check
            ;;
        cleanup)
            cleanup
            ;;
        help|--help|-h)
            show_help
            ;;
        *)
            log_error "Unknown command: $command"
            show_help
            exit 1
            ;;
    esac
}

# Run main function if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
