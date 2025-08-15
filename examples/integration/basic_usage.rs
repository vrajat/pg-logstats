//! Basic usage examples for the pg-loggrep library
//!
//! This example demonstrates how to use pg-loggrep as a library in your Rust applications.

use pg_loggrep::{
    parsers::stderr::StderrParser,
    analytics::{queries::analyze_queries, timing::analyze_timing},
    output::{json::JsonFormatter, text::TextFormatter},
    LogEntry, AnalysisResult, PgLoggrepError,
};
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("pg-loggrep Library Usage Examples");
    println!("=================================\n");

    // Example 1: Basic log parsing
    basic_parsing_example()?;

    // Example 2: File processing
    file_processing_example()?;

    // Example 3: Analysis and output formatting
    analysis_and_formatting_example()?;

    // Example 4: Custom analysis workflow
    custom_workflow_example()?;

    Ok(())
}

/// Example 1: Basic log parsing from individual lines
fn basic_parsing_example() -> Result<(), PgLoggrepError> {
    println!("1. Basic Log Parsing");
    println!("-------------------");

    let parser = StderrParser::new();

    let sample_lines = vec![
        "2024-01-15 10:30:15.123 UTC [12345]: [1-1] user=postgres,db=testdb LOG:  connection received: host=127.0.0.1 port=54321",
        "2024-01-15 10:30:15.125 UTC [12345]: [3-1] user=postgres,db=testdb LOG:  statement: SELECT * FROM users WHERE id = 1;",
        "2024-01-15 10:30:15.126 UTC [12345]: [3-2] user=postgres,db=testdb LOG:  duration: 0.123 ms",
        "2024-01-15 10:30:16.205 UTC [12346]: [3-1] user=app_user,db=production LOG:  statement: INSERT INTO orders (customer_id, total) VALUES (123, 99.99);",
        "2024-01-15 10:30:16.208 UTC [12346]: [3-2] user=app_user,db=production LOG:  duration: 2.456 ms",
    ];

    let mut entries = Vec::new();

    for line in sample_lines {
        match parser.parse_line(line)? {
            Some(entry) => {
                println!("Parsed: {} - {}", entry.level, entry.message);
                if let Some(query) = &entry.query {
                    println!("  Query: {}", query);
                }
                if let Some(duration) = entry.duration {
                    println!("  Duration: {}ms", duration);
                }
                entries.push(entry);
            }
            None => println!("Skipped line: {}", line),
        }
    }

    println!("Parsed {} log entries\n", entries.len());
    Ok(())
}

/// Example 2: Processing log files
fn file_processing_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("2. File Processing");
    println!("-----------------");

    // Create a sample log file for demonstration
    let sample_log_content = r#"2024-01-15 10:30:15.123 UTC [12345]: [1-1] user=postgres,db=testdb LOG:  connection received: host=127.0.0.1 port=54321
2024-01-15 10:30:15.125 UTC [12345]: [3-1] user=postgres,db=testdb LOG:  statement: SELECT * FROM users WHERE id = 1;
2024-01-15 10:30:15.126 UTC [12345]: [3-2] user=postgres,db=testdb LOG:  duration: 0.123 ms
2024-01-15 10:30:16.205 UTC [12346]: [3-1] user=app_user,db=production LOG:  statement: INSERT INTO orders (customer_id, total) VALUES (123, 99.99);
2024-01-15 10:30:16.208 UTC [12346]: [3-2] user=app_user,db=production LOG:  duration: 2.456 ms
2024-01-15 10:30:17.305 UTC [12347]: [3-1] user=readonly,db=analytics LOG:  statement: SELECT COUNT(*) FROM user_sessions WHERE created_at >= '2024-01-01';
2024-01-15 10:30:17.312 UTC [12347]: [3-2] user=readonly,db=analytics LOG:  duration: 6.789 ms"#;

    let temp_file = "temp_sample.log";
    fs::write(temp_file, sample_log_content)?;

    // Process the file
    let entries = process_log_file(temp_file)?;
    println!("Processed {} entries from file", entries.len());

    // Clean up
    fs::remove_file(temp_file)?;
    println!();

    Ok(())
}

/// Example 3: Analysis and output formatting
fn analysis_and_formatting_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("3. Analysis and Output Formatting");
    println!("--------------------------------");

    // Create sample data
    let entries = create_sample_entries();

    // Perform analysis
    let query_analysis = analyze_queries(&entries);
    let timing_analysis = analyze_timing(&entries);

    // Create analysis result
    let analysis_result = AnalysisResult {
        total_entries: entries.len(),
        query_types: query_analysis.query_types,
        performance_metrics: timing_analysis.performance_metrics,
        slow_queries: query_analysis.slow_queries,
        error_summary: query_analysis.error_summary,
        time_range: timing_analysis.time_range,
    };

    // Format as text
    let text_formatter = TextFormatter::new();
    let text_output = text_formatter.format(&analysis_result)?;
    println!("Text Output:");
    println!("{}", text_output);

    // Format as JSON
    let json_formatter = JsonFormatter::new();
    let json_output = json_formatter.format(&analysis_result)?;
    println!("JSON Output:");
    println!("{}\n", json_output);

    Ok(())
}

/// Example 4: Custom analysis workflow
fn custom_workflow_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("4. Custom Analysis Workflow");
    println!("---------------------------");

    let entries = create_sample_entries();

    // Custom analysis: Find queries by database
    let mut db_queries = std::collections::HashMap::new();
    for entry in &entries {
        if let (Some(query), Some(database)) = (&entry.query, &entry.database) {
            db_queries.entry(database.clone())
                .or_insert_with(Vec::new)
                .push(query.clone());
        }
    }

    println!("Queries by database:");
    for (db, queries) in db_queries {
        println!("  {}: {} queries", db, queries.len());
        for (i, query) in queries.iter().take(2).enumerate() {
            println!("    {}. {}", i + 1, query);
        }
        if queries.len() > 2 {
            println!("    ... and {} more", queries.len() - 2);
        }
    }

    // Custom analysis: Performance by user
    let mut user_performance = std::collections::HashMap::new();
    for entry in &entries {
        if let (Some(user), Some(duration)) = (&entry.user, entry.duration) {
            let stats = user_performance.entry(user.clone())
                .or_insert_with(|| (0, 0.0, f64::MAX, 0.0));
            stats.0 += 1; // count
            stats.1 += duration; // total duration
            stats.2 = stats.2.min(duration); // min duration
            stats.3 = stats.3.max(duration); // max duration
        }
    }

    println!("\nPerformance by user:");
    for (user, (count, total, min, max)) in user_performance {
        let avg = total / count as f64;
        println!("  {}: {} queries, avg: {:.2}ms, min: {:.2}ms, max: {:.2}ms",
                 user, count, avg, min, max);
    }

    Ok(())
}

/// Helper function to process a log file
fn process_log_file(file_path: &str) -> Result<Vec<LogEntry>, Box<dyn std::error::Error>> {
    let parser = StderrParser::new();
    let content = fs::read_to_string(file_path)?;
    let mut entries = Vec::new();

    for line in content.lines() {
        if let Some(entry) = parser.parse_line(line)? {
            entries.push(entry);
        }
    }

    Ok(entries)
}

/// Helper function to create sample log entries for testing
fn create_sample_entries() -> Vec<LogEntry> {
    use chrono::{DateTime, Utc};

    vec![
        LogEntry {
            timestamp: "2024-01-15T10:30:15.123Z".parse::<DateTime<Utc>>().unwrap(),
            level: pg_loggrep::LogLevel::Info,
            message: "statement: SELECT * FROM users WHERE id = 1;".to_string(),
            query: Some("SELECT * FROM users WHERE id = 1;".to_string()),
            duration: Some(0.123),
            connection_id: Some("12345".to_string()),
            database: Some("testdb".to_string()),
            user: Some("postgres".to_string()),
        },
        LogEntry {
            timestamp: "2024-01-15T10:30:16.205Z".parse::<DateTime<Utc>>().unwrap(),
            level: pg_loggrep::LogLevel::Info,
            message: "statement: INSERT INTO orders (customer_id, total) VALUES (123, 99.99);".to_string(),
            query: Some("INSERT INTO orders (customer_id, total) VALUES (123, 99.99);".to_string()),
            duration: Some(2.456),
            connection_id: Some("12346".to_string()),
            database: Some("production".to_string()),
            user: Some("app_user".to_string()),
        },
        LogEntry {
            timestamp: "2024-01-15T10:30:17.305Z".parse::<DateTime<Utc>>().unwrap(),
            level: pg_loggrep::LogLevel::Info,
            message: "statement: SELECT COUNT(*) FROM user_sessions WHERE created_at >= '2024-01-01';".to_string(),
            query: Some("SELECT COUNT(*) FROM user_sessions WHERE created_at >= '2024-01-01';".to_string()),
            duration: Some(6.789),
            connection_id: Some("12347".to_string()),
            database: Some("analytics".to_string()),
            user: Some("readonly".to_string()),
        },
        LogEntry {
            timestamp: "2024-01-15T10:30:18.405Z".parse::<DateTime<Utc>>().unwrap(),
            level: pg_loggrep::LogLevel::Info,
            message: "statement: UPDATE users SET last_login = NOW() WHERE id = 123;".to_string(),
            query: Some("UPDATE users SET last_login = NOW() WHERE id = 123;".to_string()),
            duration: Some(1.234),
            connection_id: Some("12348".to_string()),
            database: Some("production".to_string()),
            user: Some("app_user".to_string()),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_parsing() {
        let result = basic_parsing_example();
        assert!(result.is_ok());
    }

    #[test]
    fn test_sample_entries_creation() {
        let entries = create_sample_entries();
        assert_eq!(entries.len(), 4);
        assert!(entries.iter().all(|e| e.query.is_some()));
        assert!(entries.iter().all(|e| e.duration.is_some()));
    }

    #[test]
    fn test_custom_analysis() {
        let entries = create_sample_entries();

        // Test database grouping
        let mut db_count = std::collections::HashMap::new();
        for entry in &entries {
            if let Some(database) = &entry.database {
                *db_count.entry(database.clone()).or_insert(0) += 1;
            }
        }

        assert_eq!(db_count.get("production"), Some(&2));
        assert_eq!(db_count.get("testdb"), Some(&1));
        assert_eq!(db_count.get("analytics"), Some(&1));
    }
}
