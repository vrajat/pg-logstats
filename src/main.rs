use chrono::Utc;
use clap::{Args, Parser, Subcommand, ValueEnum};
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, error, info, warn};
use pg_logstats::{
    normalize_log_entries, query_family_findings, slow_query_diff_findings, Correlator,
    EventSourceKind, Finding, FindingSet, JsonFormatter, PgLogstatsError, ProcessOrderCorrelator,
    Result, SlowQueryDiffOptions, StderrLogFormat, StderrParser, TextFormatter,
};
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::process::Command as ProcessCommand;
use std::time::Instant;

#[derive(Debug, Parser)]
#[clap(
    name = "pg-logstats",
    version,
    about = "A PostgreSQL log investigation CLI for top query families, slow-query diffs, and follow-up SQL"
)]
struct Arguments {
    #[clap(subcommand)]
    command: Command,

    /// Output format for results
    #[clap(long, global = true, value_enum, default_value = "text")]
    output_format: OutputFormat,

    /// Input log format. auto supports local PostgreSQL stderr and AWS RDS logs.
    #[clap(long, global = true, value_enum, default_value = "auto")]
    input_format: InputFormat,

    /// Write results to a file. Use `-` to force stdout.
    #[clap(short = 'o', long, global = true, value_name = "PATH")]
    outfile: Option<String>,

    /// Directory to prepend to `--outfile`
    #[clap(short = 'O', long, global = true, value_name = "DIR")]
    outdir: Option<String>,

    /// Suppress progress output and the completion footer
    #[clap(short = 'q', long, global = true)]
    quiet: bool,
}

#[derive(Debug, Args)]
struct LogInputArgs {
    /// Directory containing PostgreSQL log files
    #[clap(long, value_name = "DIR")]
    log_dir: Option<PathBuf>,

    /// CloudWatch Logs group to read PostgreSQL log events from
    #[clap(long, value_name = "LOG_GROUP", conflicts_with = "rds_instance")]
    cloudwatch_log_group: Option<String>,

    /// RDS instance identifier; resolves to /aws/rds/instance/<id>/postgresql
    #[clap(
        long,
        value_name = "DB_INSTANCE",
        conflicts_with = "cloudwatch_log_group"
    )]
    rds_instance: Option<String>,

    /// Start time for CloudWatch input, as RFC3339 or a relative window like 15m, 2h, 1d
    #[clap(long, value_name = "TIME", default_value = "1h")]
    since: String,

    /// End time for CloudWatch input, as RFC3339. Defaults to now.
    #[clap(long, value_name = "TIME")]
    until: Option<String>,

    /// Optional CloudWatch Logs filter pattern
    #[clap(long, value_name = "PATTERN")]
    cloudwatch_filter_pattern: Option<String>,

    /// Maximum CloudWatch filter-log-events pages to read
    #[clap(long, value_name = "N", default_value_t = 20)]
    cloudwatch_max_pages: usize,

    /// AWS region for CloudWatch input
    #[clap(long, value_name = "REGION")]
    aws_region: Option<String>,

    /// AWS profile for CloudWatch input
    #[clap(long, value_name = "PROFILE")]
    aws_profile: Option<String>,

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

impl LogInputArgs {
    fn cloudwatch_log_group_name(&self) -> Option<String> {
        if let Some(log_group) = &self.cloudwatch_log_group {
            return Some(log_group.clone());
        }

        self.rds_instance
            .as_ref()
            .map(|instance| format!("/aws/rds/instance/{instance}/postgresql"))
    }

    fn uses_cloudwatch(&self) -> bool {
        self.cloudwatch_log_group.is_some() || self.rds_instance.is_some()
    }
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
    /// Print follow-up SQL for a finding from a findings JSON file
    SuggestSql {
        /// Findings JSON file produced by pg-logstats
        #[clap(long, value_name = "PATH")]
        findings_file: PathBuf,

        /// Select a finding by its exact finding id
        #[clap(long, value_name = "FINDING_ID", conflicts_with = "rank")]
        finding_id: Option<String>,

        /// Select a finding by rank within the findings output
        #[clap(long, value_name = "N", conflicts_with = "finding_id")]
        rank: Option<usize>,
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

#[derive(Debug, ValueEnum, Clone, Copy)]
enum InputFormat {
    /// Auto-detect among supported stderr-compatible formats.
    Auto,
    /// Local PostgreSQL stderr logs using the pg-logstats supported prefix.
    Stderr,
    /// Amazon RDS PostgreSQL logs using `%t:%r:%u@%d:[%p]:`.
    Rds,
}

impl InputFormat {
    fn stderr_log_format(self) -> StderrLogFormat {
        match self {
            Self::Auto => StderrLogFormat::Auto,
            Self::Stderr => StderrLogFormat::Standard,
            Self::Rds => StderrLogFormat::AwsRds,
        }
    }

    fn event_source_kind(self) -> EventSourceKind {
        match self {
            Self::Rds => EventSourceKind::AwsRds,
            Self::Auto | Self::Stderr => EventSourceKind::Stderr,
        }
    }
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
        Command::SuggestSql {
            findings_file,
            finding_id,
            rank,
        } => run_suggest_sql_command(args, findings_file, finding_id.as_deref(), *rank),
    }
}

fn load_default_log_entries(
    args: &Arguments,
    input: &LogInputArgs,
    parser: &StderrParser,
) -> Result<Vec<pg_logstats::LogEntry>> {
    if input.uses_cloudwatch() {
        let entries = process_cloudwatch_input(input, parser)?;
        if entries.is_empty() {
            warn!("No CloudWatch log events were successfully parsed");
            process::exit(1);
        }

        info!("Total CloudWatch entries parsed: {}", entries.len());
        return Ok(entries);
    }

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
    let findings = run_top_query_families(&all_entries, limit, source_kind_for_input(args, input))?;
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
    let (findings, total_entries) = run_slow_queries_diff(
        baseline,
        target,
        parser,
        sample_size,
        options,
        args.input_format.event_source_kind(),
    )?;
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
        Command::SuggestSql {
            findings_file,
            finding_id,
            rank,
        } => validate_suggest_sql_args(findings_file, finding_id.as_deref(), *rank)?,
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
    if input.uses_cloudwatch() {
        validate_cloudwatch_input_args(input)?;
        return validate_sample_size(input.sample_size);
    }

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

fn validate_cloudwatch_input_args(input: &LogInputArgs) -> Result<()> {
    if input.cloudwatch_max_pages == 0 {
        return Err(PgLogstatsError::Configuration {
            message: "CloudWatch max pages must be greater than 0".to_string(),
            field: Some("cloudwatch_max_pages".to_string()),
        });
    }

    if input.log_dir.is_some() || input.logfile_list.is_some() || !input.log_files.is_empty() {
        return Err(PgLogstatsError::Configuration {
            message: "CloudWatch input cannot be combined with local log files".to_string(),
            field: Some("cloudwatch_input".to_string()),
        });
    }

    Ok(())
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

fn validate_suggest_sql_args(
    findings_file: &Path,
    finding_id: Option<&str>,
    rank: Option<usize>,
) -> Result<()> {
    if !findings_file.exists() {
        return Err(PgLogstatsError::Configuration {
            message: format!("Findings file does not exist: {}", findings_file.display()),
            field: Some("findings_file".to_string()),
        });
    }

    if !findings_file.is_file() {
        return Err(PgLogstatsError::Configuration {
            message: format!("Findings path is not a file: {}", findings_file.display()),
            field: Some("findings_file".to_string()),
        });
    }

    if finding_id.is_none() && rank.is_none() {
        return Err(PgLogstatsError::Configuration {
            message: "Specify either --finding-id or --rank".to_string(),
            field: Some("finding_selector".to_string()),
        });
    }

    if matches!(rank, Some(0)) {
        return Err(PgLogstatsError::Configuration {
            message: "Rank must be greater than 0".to_string(),
            field: Some("rank".to_string()),
        });
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

fn initialize_parser(args: &Arguments) -> Result<StderrParser> {
    debug!("Initializing stderr parser for {:?}", args.input_format);
    Ok(StderrParser::with_format(
        args.input_format.stderr_log_format(),
    ))
}

fn source_kind_for_input(args: &Arguments, input: &LogInputArgs) -> EventSourceKind {
    if input.uses_cloudwatch() && matches!(args.input_format, InputFormat::Auto) {
        return EventSourceKind::AwsRds;
    }

    args.input_format.event_source_kind()
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

fn process_cloudwatch_input(
    input: &LogInputArgs,
    parser: &StderrParser,
) -> Result<Vec<pg_logstats::LogEntry>> {
    let log_group = input
        .cloudwatch_log_group_name()
        .expect("validated CloudWatch input should have a log group");
    let events = fetch_cloudwatch_events(input, &log_group)?;
    let mut lines: Vec<String> = events
        .into_iter()
        .filter_map(|event| event.message)
        .flat_map(|message| message.lines().map(str::to_string).collect::<Vec<String>>())
        .collect();

    if let Some(sample_size) = input.sample_size {
        lines.truncate(sample_size);
    }

    parser.parse_lines(&lines)
}

#[derive(Debug, serde::Deserialize)]
struct CloudWatchFilterEventsResponse {
    #[serde(default)]
    events: Vec<CloudWatchLogEvent>,
    #[serde(rename = "nextToken")]
    next_token: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct CloudWatchLogEvent {
    message: Option<String>,
}

fn fetch_cloudwatch_events(
    input: &LogInputArgs,
    log_group: &str,
) -> Result<Vec<CloudWatchLogEvent>> {
    let start_time = parse_cloudwatch_start_time_ms(&input.since)?;
    let end_time = parse_cloudwatch_end_time_ms(input.until.as_deref())?;
    if end_time <= start_time {
        return Err(PgLogstatsError::Configuration {
            message: "--until must be after --since".to_string(),
            field: Some("until".to_string()),
        });
    }

    let mut events = Vec::new();
    let mut next_token: Option<String> = None;
    let mut previous_token: Option<String> = None;

    for _page in 0..input.cloudwatch_max_pages {
        let response = run_aws_logs_filter_log_events(
            input,
            log_group,
            start_time,
            end_time,
            next_token.as_deref(),
        )?;
        events.extend(response.events);

        if response.next_token.is_none() || response.next_token == previous_token {
            break;
        }

        previous_token = next_token;
        next_token = response.next_token;
    }

    Ok(events)
}

fn run_aws_logs_filter_log_events(
    input: &LogInputArgs,
    log_group: &str,
    start_time: i64,
    end_time: i64,
    next_token: Option<&str>,
) -> Result<CloudWatchFilterEventsResponse> {
    let aws_cli = std::env::var("PG_LOGSTATS_AWS_CLI").unwrap_or_else(|_| "aws".to_string());
    let mut command = ProcessCommand::new(aws_cli);
    command
        .arg("logs")
        .arg("filter-log-events")
        .arg("--log-group-name")
        .arg(log_group)
        .arg("--start-time")
        .arg(start_time.to_string())
        .arg("--end-time")
        .arg(end_time.to_string())
        .arg("--output")
        .arg("json")
        .arg("--no-cli-pager");

    if let Some(filter_pattern) = &input.cloudwatch_filter_pattern {
        command.arg("--filter-pattern").arg(filter_pattern);
    }

    if let Some(region) = &input.aws_region {
        command.arg("--region").arg(region);
    }

    if let Some(profile) = &input.aws_profile {
        command.arg("--profile").arg(profile);
    }

    if let Some(next_token) = next_token {
        command.arg("--next-token").arg(next_token);
    }

    let output = command
        .output()
        .map_err(|err| PgLogstatsError::Configuration {
            message: format!("Failed to run AWS CLI for CloudWatch input: {err}"),
            field: Some("aws_cli".to_string()),
        })?;

    if !output.status.success() {
        return Err(PgLogstatsError::Configuration {
            message: format!(
                "AWS CLI CloudWatch query failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ),
            field: Some("cloudwatch".to_string()),
        });
    }

    serde_json::from_slice(&output.stdout).map_err(PgLogstatsError::Serialization)
}

fn parse_cloudwatch_start_time_ms(value: &str) -> Result<i64> {
    if let Some(duration_ms) = parse_relative_duration_ms(value) {
        return Ok(Utc::now().timestamp_millis() - duration_ms);
    }

    parse_rfc3339_millis(value, "since")
}

fn parse_cloudwatch_end_time_ms(value: Option<&str>) -> Result<i64> {
    match value {
        Some(value) => parse_rfc3339_millis(value, "until"),
        None => Ok(Utc::now().timestamp_millis()),
    }
}

fn parse_relative_duration_ms(value: &str) -> Option<i64> {
    let (number, unit) = value.split_at(value.len().checked_sub(1)?);
    let amount: i64 = number.parse().ok()?;
    let multiplier = match unit {
        "m" => 60_000,
        "h" => 3_600_000,
        "d" => 86_400_000,
        _ => return None,
    };

    amount.checked_mul(multiplier)
}

fn parse_rfc3339_millis(value: &str, field: &str) -> Result<i64> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|timestamp| timestamp.timestamp_millis())
        .map_err(|err| PgLogstatsError::Configuration {
            message: format!("Invalid --{field} time `{value}`: {err}"),
            field: Some(field.to_string()),
        })
}

fn run_top_query_families(
    entries: &[pg_logstats::LogEntry],
    limit: usize,
    source_kind: EventSourceKind,
) -> Result<pg_logstats::FindingSet> {
    info!(
        "Building top query-family findings from {} entries",
        entries.len()
    );
    let events = normalize_log_entries(entries, source_kind);
    let executions = ProcessOrderCorrelator.correlate(&events);

    Ok(query_family_findings(&executions, limit))
}

fn run_slow_queries_diff(
    baseline: &Path,
    target: &Path,
    parser: &StderrParser,
    sample_size: Option<usize>,
    options: SlowQueryDiffOptions,
    source_kind: EventSourceKind,
) -> Result<(pg_logstats::FindingSet, usize)> {
    info!(
        "Building slow-query diff findings from baseline {} and target {}",
        baseline.display(),
        target.display()
    );

    let baseline_entries = process_log_paths(baseline, parser, sample_size)?;
    let target_entries = process_log_paths(target, parser, sample_size)?;

    let baseline_events = normalize_log_entries(&baseline_entries, source_kind);
    let target_events = normalize_log_entries(&target_entries, source_kind);
    let baseline_executions = ProcessOrderCorrelator.correlate(&baseline_events);
    let target_executions = ProcessOrderCorrelator.correlate(&target_events);

    let findings = slow_query_diff_findings(&baseline_executions, &target_executions, options);
    let total_entries = baseline_entries.len() + target_entries.len();

    Ok((findings, total_entries))
}

fn run_suggest_sql_command(
    args: &Arguments,
    findings_file: &Path,
    finding_id: Option<&str>,
    rank: Option<usize>,
) -> Result<()> {
    let findings = load_findings_file(findings_file)?;
    let finding = select_finding(&findings, finding_id, rank)?;

    if finding.next_sql.is_empty() {
        return Err(PgLogstatsError::Configuration {
            message: format!("No suggested SQL is available for {}", finding.finding_id),
            field: Some("finding_id".to_string()),
        });
    }

    output_suggested_sql(args, finding)
}

fn load_findings_file(path: &Path) -> Result<FindingSet> {
    let content = fs::read_to_string(path)?;
    serde_json::from_str(&content).map_err(PgLogstatsError::Serialization)
}

fn select_finding<'a>(
    findings: &'a FindingSet,
    finding_id: Option<&str>,
    rank: Option<usize>,
) -> Result<&'a Finding> {
    if let Some(finding_id) = finding_id {
        return findings
            .findings
            .iter()
            .find(|finding| finding.finding_id == finding_id)
            .ok_or_else(|| PgLogstatsError::Configuration {
                message: format!("Finding id not found: {}", finding_id),
                field: Some("finding_id".to_string()),
            });
    }

    if let Some(rank) = rank {
        return findings
            .findings
            .iter()
            .find(|finding| finding.rank == rank)
            .ok_or_else(|| PgLogstatsError::Configuration {
                message: format!("Finding rank not found: {}", rank),
                field: Some("rank".to_string()),
            });
    }

    Err(PgLogstatsError::Configuration {
        message: "Specify either --finding-id or --rank".to_string(),
        field: Some("finding_selector".to_string()),
    })
}

fn output_suggested_sql(args: &Arguments, finding: &Finding) -> Result<()> {
    match args.output_format {
        OutputFormat::Json => {
            let output = serde_json::to_string_pretty(&json!({
                "finding_id": finding.finding_id,
                "rank": finding.rank,
                "kind": finding.kind,
                "title": finding.title,
                "next_sql": finding.next_sql,
            }))
            .map_err(PgLogstatsError::Serialization)?;
            write_or_print_output(output, args)
        }
        OutputFormat::Text => {
            let mut output = String::new();
            output.push_str(&format!(
                "#{} [{}] {}\n",
                finding.rank, finding.finding_id, finding.title
            ));
            for statement in &finding.next_sql {
                output.push_str(statement);
                output.push('\n');
            }
            write_or_print_output(output, args)
        }
    }
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
            let output_path = if let Some(outdir) = &args.outdir {
                Path::new(outdir).join(outfile)
            } else {
                PathBuf::from(outfile)
            };
            fs::write(&output_path, output)?;
            info!("Results written to {}", output_path.display());
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
