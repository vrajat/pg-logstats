use clap::{Args, Parser, Subcommand, ValueEnum};
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, error, info, warn};
use pg_logstats::{
    normalize_log_entries, query_family_findings, slow_query_diff_findings, Correlator,
    EventSourceKind, JsonFormatter, PgLogstatsError, ProcessOrderCorrelator, Result,
    SlowQueryDiffOptions, StderrParser, TextFormatter,
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
    #[clap(subcommand)]
    command: Command,

    /// Output format for results
    #[clap(long, value_enum, default_value = "text")]
    output_format: OutputFormat,

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

#[derive(Debug, Args)]
struct LogInputArgs {
    /// Directory containing PostgreSQL log files
    #[clap(long, value_name = "DIR")]
    log_dir: Option<PathBuf>,

    /// Limit analysis to first N lines of each file (for large files)
    #[clap(long, value_name = "N")]
    sample_size: Option<usize>,

    /// file containing a list of log file to parse.
    #[clap(short = 'L', long, value_name = "logfile-list")]
    logfile_list: Option<String>,

    /// Log files to analyze
    #[clap(value_name = "LOG_FILES")]
    log_files: Vec<String>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Investigation-oriented top findings
    Top {
        #[clap(subcommand)]
        command: TopCommand,
    },
    /// Slow-query investigation workflows
    SlowQueries {
        #[clap(subcommand)]
        command: SlowQueriesCommand,
    },
}

#[derive(Debug, Subcommand)]
enum TopCommand {
    /// Rank query families by total runtime in one log window
    QueryFamilies {
        /// Maximum number of query-family findings to emit
        #[clap(long, default_value_t = 10)]
        limit: usize,

        #[clap(flatten)]
        input: LogInputArgs,
    },
}

#[derive(Debug, Subcommand)]
enum SlowQueriesCommand {
    /// Compare target logs against explicit baseline logs
    Diff {
        /// Baseline log file or directory
        #[clap(long, value_name = "PATH")]
        baseline: PathBuf,

        /// Target log file or directory
        #[clap(long, value_name = "PATH")]
        target: PathBuf,

        /// Limit analysis to first N lines of each file in each window
        #[clap(long, value_name = "N")]
        sample_size: Option<usize>,

        /// Maximum number of findings to emit
        #[clap(long, default_value_t = 10)]
        limit: usize,

        /// Minimum target executions for a query family to be eligible
        #[clap(long, default_value_t = 1)]
        min_target_count: u64,

        /// Minimum target total runtime in milliseconds
        #[clap(long, default_value_t = 0.0)]
        min_target_total_ms: f64,

        /// Minimum p95 regression in milliseconds
        #[clap(long, default_value_t = 0.0)]
        min_p95_delta_ms: f64,
    },
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

    // Initialize parser based on format
    let parser = initialize_parser(&args)?;

    run_command(&args, &parser)?;

    let elapsed = start_time.elapsed();
    if !args.quiet {
        println!("Analysis completed in {:.2}s", elapsed.as_secs_f64());
    }

    Ok(())
}

fn run_command(args: &Arguments, parser: &StderrParser) -> Result<()> {
    match &args.command {
        Command::Top {
            command: TopCommand::QueryFamilies { limit, input },
        } => run_top_query_families_command(args, parser, input, *limit),
        Command::SlowQueries {
            command:
                SlowQueriesCommand::Diff {
                    baseline,
                    target,
                    sample_size,
                    limit,
                    min_target_count,
                    min_target_total_ms,
                    min_p95_delta_ms,
                },
        } => run_slow_queries_diff_command(
            args,
            parser,
            baseline,
            target,
            *sample_size,
            SlowQueryDiffOptions {
                limit: *limit,
                min_target_count: *min_target_count,
                min_target_total_ms: *min_target_total_ms,
                min_p95_delta_ms: *min_p95_delta_ms,
            },
        ),
    }
}

fn load_default_log_entries(
    args: &Arguments,
    input: &LogInputArgs,
    parser: &StderrParser,
) -> Result<Vec<pg_logstats::LogEntry>> {
    // Initialize progress bar if not in quiet mode
    let progress_bar = if !args.quiet {
        Some(create_progress_bar())
    } else {
        None
    };

    // Discover log files
    let log_files = discover_log_files(input)?;

    if log_files.is_empty() {
        error!("No log files found to process");
        process::exit(1);
    }

    info!("Found {} log files to process", log_files.len());

    // Process log files with progress indication
    let mut all_entries = Vec::new();

    for (index, log_file) in log_files.iter().enumerate() {
        if let Some(pb) = &progress_bar {
            pb.set_message(format!("Processing {}", log_file.display()));
            pb.set_position(index as u64);
        }

        match process_log_file(log_file, parser, input.sample_size) {
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
    Ok(all_entries)
}

fn run_top_query_families_command(
    args: &Arguments,
    parser: &StderrParser,
    input: &LogInputArgs,
    limit: usize,
) -> Result<()> {
    let all_entries = load_default_log_entries(args, input, parser)?;
    let findings = run_top_query_families(&all_entries, limit)?;
    output_findings(&findings, args, &all_entries)
}

fn run_slow_queries_diff_command(
    args: &Arguments,
    parser: &StderrParser,
    baseline: &Path,
    target: &Path,
    sample_size: Option<usize>,
    options: SlowQueryDiffOptions,
) -> Result<()> {
    let (findings, total_entries) =
        run_slow_queries_diff(baseline, target, parser, sample_size, options)?;
    output_findings_with_entry_count(&findings, args, total_entries)
}

fn validate_arguments(args: &Arguments) -> Result<()> {
    match &args.command {
        Command::Top {
            command: TopCommand::QueryFamilies { input, .. },
        } => validate_log_input_args(input)?,
        Command::SlowQueries {
            command: SlowQueriesCommand::Diff { sample_size, .. },
        } => validate_sample_size(*sample_size)?,
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

fn validate_log_input_args(input: &LogInputArgs) -> Result<()> {
    if let Some(log_dir) = &input.log_dir {
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

    validate_sample_size(input.sample_size)
}

fn validate_sample_size(sample_size: Option<usize>) -> Result<()> {
    if let Some(sample_size) = sample_size {
        if sample_size == 0 {
            return Err(PgLogstatsError::Configuration {
                message: "Sample size must be greater than 0".to_string(),
                field: Some("sample_size".to_string()),
            });
        }
    }

    Ok(())
}

fn discover_log_files(input: &LogInputArgs) -> Result<Vec<PathBuf>> {
    let mut log_files = Vec::new();

    // If log_dir is specified, discover files in that directory
    if let Some(log_dir) = &input.log_dir {
        discover_files_in_directory(log_dir, &mut log_files)?;
    }

    // Add explicitly specified log files
    for file_pattern in &input.log_files {
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
    if let Some(logfile_list) = &input.logfile_list {
        let list_content = fs::read_to_string(logfile_list).map_err(PgLogstatsError::Io)?;

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

fn discover_log_files_for_path(path: &Path) -> Result<Vec<PathBuf>> {
    if !path.exists() {
        return Err(PgLogstatsError::Configuration {
            message: format!("Log path does not exist: {}", path.display()),
            field: Some("path".to_string()),
        });
    }

    let mut log_files = Vec::new();
    if path.is_file() {
        log_files.push(path.to_path_buf());
    } else if path.is_dir() {
        discover_files_in_directory(path, &mut log_files)?;
    } else {
        return Err(PgLogstatsError::Configuration {
            message: format!("Log path is neither file nor directory: {}", path.display()),
            field: Some("path".to_string()),
        });
    }

    log_files.sort();
    log_files.dedup();
    Ok(log_files)
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
    sample_size: Option<usize>,
) -> Result<Vec<pg_logstats::LogEntry>> {
    let content = fs::read_to_string(log_file)?;
    let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

    // Apply sample size limit if specified
    let lines_to_process = if let Some(sample_size) = sample_size {
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

fn process_log_paths(
    path: &Path,
    parser: &StderrParser,
    sample_size: Option<usize>,
) -> Result<Vec<pg_logstats::LogEntry>> {
    let log_files = discover_log_files_for_path(path)?;
    if log_files.is_empty() {
        return Err(PgLogstatsError::Configuration {
            message: format!("No log files found under {}", path.display()),
            field: Some("path".to_string()),
        });
    }

    let mut all_entries = Vec::new();
    for log_file in log_files {
        let mut entries = process_log_file(&log_file, parser, sample_size)?;
        all_entries.append(&mut entries);
    }

    Ok(all_entries)
}

fn run_top_query_families(
    entries: &[pg_logstats::LogEntry],
    limit: usize,
) -> Result<pg_logstats::FindingSet> {
    info!(
        "Building top query-family findings from {} entries",
        entries.len()
    );
    let events = normalize_log_entries(entries, EventSourceKind::Stderr);
    let executions = ProcessOrderCorrelator.correlate(&events);

    Ok(query_family_findings(&executions, limit))
}

fn run_slow_queries_diff(
    baseline: &Path,
    target: &Path,
    parser: &StderrParser,
    sample_size: Option<usize>,
    options: SlowQueryDiffOptions,
) -> Result<(pg_logstats::FindingSet, usize)> {
    info!(
        "Building slow-query diff findings from baseline {} and target {}",
        baseline.display(),
        target.display()
    );

    let baseline_entries = process_log_paths(baseline, parser, sample_size)?;
    let target_entries = process_log_paths(target, parser, sample_size)?;

    let baseline_events = normalize_log_entries(&baseline_entries, EventSourceKind::Stderr);
    let target_events = normalize_log_entries(&target_entries, EventSourceKind::Stderr);
    let baseline_executions = ProcessOrderCorrelator.correlate(&baseline_events);
    let target_executions = ProcessOrderCorrelator.correlate(&target_events);

    let findings = slow_query_diff_findings(&baseline_executions, &target_executions, options);
    let total_entries = baseline_entries.len() + target_entries.len();

    Ok((findings, total_entries))
}

fn output_findings(
    findings: &pg_logstats::FindingSet,
    args: &Arguments,
    entries: &[pg_logstats::LogEntry],
) -> Result<()> {
    output_findings_with_entry_count(findings, args, entries.len())
}

fn output_findings_with_entry_count(
    findings: &pg_logstats::FindingSet,
    args: &Arguments,
    total_log_entries: usize,
) -> Result<()> {
    match args.output_format {
        OutputFormat::Json => {
            let formatter = JsonFormatter::new().with_pretty(true).with_metadata(
                env!("CARGO_PKG_VERSION"),
                vec![],
                total_log_entries,
            );

            let output = formatter.format_findings(findings)?;
            write_or_print_output(output, args)?;
        }
        OutputFormat::Text => {
            let formatter = TextFormatter::new();
            let output = formatter.format_findings(findings)?;
            write_or_print_output(output, args)?;
        }
    }

    Ok(())
}

fn write_or_print_output(output: String, args: &Arguments) -> Result<()> {
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
