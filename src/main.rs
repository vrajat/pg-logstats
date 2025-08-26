use clap::{Parser, ValueEnum};
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, error, info, warn};
use pg_logstats::{
    JsonFormatter, PgLogstatsError, QueryAnalyzer, Result, StderrParser, TextFormatter,
    TimingAnalyzer,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::time::Instant;

#[derive(Debug, Parser)]
#[clap(
    name = "pg-logstats",
    version,
    about = "A fast PostgreSQL log analysis tool"
)]
struct Arguments {
    /// Log files or directory to analyze (supports glob patterns)
    #[clap(value_name = "LOG_FILES")]
    log_files: Vec<String>,

    // Phase 1 Features
    /// Directory containing PostgreSQL log files
    #[clap(long, value_name = "DIR")]
    log_dir: Option<PathBuf>,

    /// Output format for results
    #[clap(long, value_enum, default_value = "text")]
    output_format: OutputFormat,

    /// Show only summary information (quick mode)
    #[clap(long)]
    quick: bool,

    /// Limit analysis to first N lines of each file (for large files)
    #[clap(long, value_name = "N")]
    sample_size: Option<usize>,

    // Existing options (keeping the most important ones)
    /// file containing a list of log file to parse.
    #[clap(short = 'L', long, value_name = "logfile-list")]
    logfile_list: Option<String>,

    /// define the filename for the output. Default depends on the output format: out.html, out.txt, out.bin, out.json or out.tsung. This option can be used multiple time to output several format. To use json output the Perl module JSON::XS must be installed, To dump output to stdout use - as filename.
    #[clap(short = 'o', long, value_name = "outfile")]
    outfile: Option<String>,

    /// directory where out file must be saved.
    #[clap(short = 'O', long, value_name = "outdir")]
    outdir: Option<String>,

    /// don't print anything to stdout, not even a progress bar.
    #[clap(short = 'q', long, value_name = "quiet")]
    quiet: bool,
}

#[derive(Debug, ValueEnum, Clone, Copy)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, ValueEnum, Clone, Copy, Default)]
enum Format {
    #[default]
    Syslog,
    Syslog2,
    Stderr,
    Jsonlog,
    Cvs,
    Pgbouncer,
    Logplex,
    Rds,
    Redshift,
}

fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    let args = Arguments::parse();
    let start_time = Instant::now();

    // Validate CLI arguments
    validate_arguments(&args)?;

    // Initialize progress bar if not in quiet mode
    let progress_bar = if !args.quiet {
        Some(create_progress_bar())
    } else {
        None
    };

    // Discover log files
    let log_files = discover_log_files(&args)?;

    if log_files.is_empty() {
        error!("No log files found to process");
        process::exit(1);
    }

    info!("Found {} log files to process", log_files.len());

    // Initialize parser based on format
    let parser = initialize_parser(&args)?;

    // Process log files with progress indication
    let mut all_entries = Vec::new();

    for (index, log_file) in log_files.iter().enumerate() {
        if let Some(pb) = &progress_bar {
            pb.set_message(format!("Processing {}", log_file.display()));
            pb.set_position(index as u64);
        }

        match process_log_file(log_file, &parser, &args) {
            Ok(mut entries) => {
                info!(
                    "Processed {} entries from {}",
                    entries.len(),
                    log_file.display()
                );
                all_entries.append(&mut entries);
            }
            Err(e) => {
                warn!("Failed to process {}: {}", log_file.display(), e);
                continue;
            }
        }
    }

    if let Some(pb) = &progress_bar {
        pb.finish_with_message("File processing complete");
    }

    if all_entries.is_empty() {
        warn!("No log entries were successfully parsed");
        process::exit(1);
    }

    info!("Total entries parsed: {}", all_entries.len());

    // Run analytics on parsed data
    let analytics_result = run_analytics(&all_entries, &args)?;

    // Output results in requested format
    output_results(&analytics_result, &args, &all_entries)?;

    let elapsed = start_time.elapsed();
    if !args.quiet {
        println!("Analysis completed in {:.2}s", elapsed.as_secs_f64());
    }

    Ok(())
}

fn validate_arguments(args: &Arguments) -> Result<()> {
    // Check if log directory exists and is readable
    if let Some(log_dir) = &args.log_dir {
        if !log_dir.exists() {
            return Err(PgLogstatsError::Configuration {
                message: format!("Log directory does not exist: {}", log_dir.display()),
                field: Some("log_dir".to_string()),
            });
        }

        if !log_dir.is_dir() {
            return Err(PgLogstatsError::Configuration {
                message: format!(
                    "Log directory path is not a directory: {}",
                    log_dir.display()
                ),
                field: Some("log_dir".to_string()),
            });
        }

        // Test readability
        match fs::read_dir(log_dir) {
            Ok(_) => {}
            Err(e) => {
                return Err(PgLogstatsError::Configuration {
                    message: format!("Cannot read log directory {}: {}", log_dir.display(), e),
                    field: Some("log_dir".to_string()),
                });
            }
        }
    }

    // Validate sample size
    if let Some(sample_size) = args.sample_size {
        if sample_size == 0 {
            return Err(PgLogstatsError::Configuration {
                message: "Sample size must be greater than 0".to_string(),
                field: Some("sample_size".to_string()),
            });
        }
    }

    // Validate output directory if specified
    if let Some(outdir) = &args.outdir {
        let outdir_path = Path::new(outdir);
        if outdir_path.exists() && !outdir_path.is_dir() {
            return Err(PgLogstatsError::Configuration {
                message: format!(
                    "Output directory path exists but is not a directory: {}",
                    outdir
                ),
                field: Some("outdir".to_string()),
            });
        }
    }

    Ok(())
}

fn discover_log_files(args: &Arguments) -> Result<Vec<PathBuf>> {
    let mut log_files = Vec::new();

    // If log_dir is specified, discover files in that directory
    if let Some(log_dir) = &args.log_dir {
        discover_files_in_directory(log_dir, &mut log_files)?;
    }

    // Add explicitly specified log files
    for file_pattern in &args.log_files {
        if let Ok(path) = PathBuf::from(file_pattern).canonicalize() {
            if path.is_file() {
                log_files.push(path);
            }
        } else {
            // Try glob pattern matching (simplified implementation)
            let path = Path::new(file_pattern);
            if path.exists() && path.is_file() {
                log_files.push(path.to_path_buf());
            }
        }
    }

    // If logfile_list is specified, read file list
    if let Some(logfile_list) = &args.logfile_list {
        let list_content = fs::read_to_string(logfile_list).map_err(|e| PgLogstatsError::Io(e))?;

        for line in list_content.lines() {
            let line = line.trim();
            if !line.is_empty() && !line.starts_with('#') {
                let path = Path::new(line);
                if path.exists() && path.is_file() {
                    log_files.push(path.to_path_buf());
                }
            }
        }
    }

    // Remove duplicates and sort
    log_files.sort();
    log_files.dedup();

    // Warn about empty files
    log_files.retain(|path| match fs::metadata(path) {
        Ok(metadata) => {
            if metadata.len() == 0 {
                warn!("Skipping empty log file: {}", path.display());
                false
            } else {
                true
            }
        }
        Err(e) => {
            warn!("Cannot read metadata for {}: {}", path.display(), e);
            false
        }
    });

    Ok(log_files)
}

fn discover_files_in_directory(dir: &Path, log_files: &mut Vec<PathBuf>) -> Result<()> {
    let entries = fs::read_dir(dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            // Check if it looks like a log file
            if let Some(extension) = path.extension() {
                let ext_str = extension.to_string_lossy().to_lowercase();
                if ext_str == "log" || ext_str == "txt" {
                    log_files.push(path);
                }
            } else if let Some(filename) = path.file_name() {
                let filename_str = filename.to_string_lossy().to_lowercase();
                if filename_str.contains("postgres") || filename_str.contains("pg") {
                    log_files.push(path);
                }
            }
        }
    }

    Ok(())
}

fn initialize_parser(_args: &Arguments) -> Result<StderrParser> {
    // For now, we'll use StderrParser as the default
    // In the future, we can add logic to choose parser based on format
    debug!("Initializing stderr parser");
    Ok(StderrParser::new())
}

fn process_log_file(
    log_file: &Path,
    parser: &StderrParser,
    args: &Arguments,
) -> Result<Vec<pg_logstats::LogEntry>> {
    let content = fs::read_to_string(log_file)?;
    let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

    // Apply sample size limit if specified
    let lines_to_process = if let Some(sample_size) = args.sample_size {
        if lines.len() > sample_size {
            info!(
                "Limiting analysis to first {} lines of {}",
                sample_size,
                log_file.display()
            );
            &lines[..sample_size]
        } else {
            &lines
        }
    } else {
        &lines
    };

    parser.parse_lines(lines_to_process)
}

fn run_analytics(
    entries: &[pg_logstats::LogEntry],
    _args: &Arguments,
) -> Result<pg_logstats::AnalysisResult> {
    info!("Running analytics on {} entries", entries.len());

    let query_analyzer = QueryAnalyzer::new();
    let timing_analyzer = TimingAnalyzer::new();

    // Run query analysis
    let analysis_result = query_analyzer.analyze(entries)?;

    // Run timing analysis
    let _timing_analysis = timing_analyzer.analyze_timing(entries)?;

    Ok(analysis_result)
}

fn output_results(
    analytics_result: &pg_logstats::AnalysisResult,
    args: &Arguments,
    entries: &[pg_logstats::LogEntry],
) -> Result<()> {
    match args.output_format {
        OutputFormat::Json => {
            let formatter = JsonFormatter::new().with_pretty(true).with_metadata(
                env!("CARGO_PKG_VERSION"),
                vec![],
                entries.len(),
            );

            let output = formatter.format(analytics_result)?;

            if let Some(outfile) = &args.outfile {
                if outfile == "-" {
                    println!("{}", output);
                } else {
                    fs::write(outfile, output)?;
                    info!("Results written to {}", outfile);
                }
            } else {
                println!("{}", output);
            }
        }
        OutputFormat::Text => {
            let formatter = TextFormatter::new();
            let output = formatter.format_query_analysis(analytics_result)?;

            if let Some(outfile) = &args.outfile {
                if outfile == "-" {
                    println!("{}", output);
                } else {
                    fs::write(outfile, output)?;
                    info!("Results written to {}", outfile);
                }
            } else {
                println!("{}", output);
            }
        }
    }

    Ok(())
}

fn create_progress_bar() -> ProgressBar {
    let pb = ProgressBar::new(100);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb
}
