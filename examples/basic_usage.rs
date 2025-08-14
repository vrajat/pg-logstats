//! Basic usage example for pg-loggrep
//!
//! This example demonstrates how to use the pg-loggrep library to parse
//! and analyze PostgreSQL log files.

use pg_loggrep::{
    StderrParser, QueryAnalyzer, TimingAnalyzer,
    JsonFormatter, TextFormatter
};
use std::fs::File;
use std::io::{self, BufRead, BufReader};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("pg-loggrep Basic Usage Example");
    println!("==============================");

    // Initialize parsers and analyzers
    let parser = StderrParser::new();
    let query_analyzer = QueryAnalyzer::new();
    let timing_analyzer = TimingAnalyzer::new();
    let json_formatter = JsonFormatter::new();
    let text_formatter = TextFormatter::new();

    // Read log file (replace with your actual log file path)
    let log_file = "demo/logs/postgresql-*.log";
    println!("Reading log file: {}", log_file);

    // Parse log entries
    let log_lines = read_log_lines(log_file)?;
    let entries = parser.parse_lines(&log_lines)?;

    println!("Parsed {} log entries", entries.len());

    // Analyze queries
    let query_analysis = query_analyzer.analyze_queries(&entries);
    println!("Query analysis completed");

    // Analyze timing
    let timing_analysis = timing_analyzer.analyze_timing(&entries);
    println!("Timing analysis completed");

    // Format results as text
    let text_output = text_formatter.format_query_analysis(&query_analysis)?;
    println!("\nQuery Analysis (Text):");
    println!("{}", text_output);

    let timing_text = text_formatter.format_timing_analysis(&timing_analysis)?;
    println!("\nTiming Analysis (Text):");
    println!("{}", timing_text);

    // Format results as JSON
    let json_output = json_formatter.format_query_analysis(&query_analysis)?;
    println!("\nQuery Analysis (JSON):");
    println!("{}", json_output);

    Ok(())
}

fn read_log_lines(file_pattern: &str) -> Result<Vec<String>, io::Error> {
    // This is a simplified example - in practice you'd want to handle
    // glob patterns and multiple files
    let mut lines = Vec::new();

    // For demo purposes, create some sample log lines
    lines.push("2024-01-01 10:00:00.123 UTC [1234]: [1-1] user=testuser,db=testdb,app=psql,client=127.0.0.1 LOG: statement: SELECT * FROM users;".to_string());
    lines.push("2024-01-01 10:00:01.456 UTC [1234]: [2-1] user=testuser,db=testdb,app=psql,client=127.0.0.1 LOG: duration: 15.234 ms".to_string());

    Ok(lines)
}
