use clap::{Args, Parser, Subcommand, ValueEnum};
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, error, info, warn};
use pg_logstats::{
    execute_run_sql,
    input::{
        discover_log_files, process_cloudwatch_input, process_log_file, process_log_paths,
        validate_file_input_args, CloudWatchInput, CloudWatchSince, CloudWatchUntil, LocalLogInput,
    },
    inspect, load_config, normalize_log_entries, parse_action_parameters, query_family_findings,
    resolve_workspace_path, run_running_queries, slow_query_diff_findings,
    top_query_families_report, workflow_slug, workspace_inspect_report_path, ActionKind,
    ActionParameterInput, Correlator, EventSourceKind, InspectReportPayload, JsonFormatter,
    NextAction, NextActionStatus, OperatingMode, OutputFormat, PgLogstatsError, PgTriageReport,
    ProcessOrderCorrelator, Result, RunSqlRequest, SlowQueryDiffOptions, TextFormatter,
    TextLogFormat, TextLogParser,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
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

    /// Workspace directory for config, inspect output, and cached results
    #[clap(long, global = true, value_name = "DIR")]
    workspace: Option<PathBuf>,

    /// Suppress progress output and the completion footer
    #[clap(short = 'q', long, global = true)]
    quiet: bool,

    /// Session ID to group reports and reconstruct the investigation graph
    #[clap(long, global = true, value_name = "SESSION_ID")]
    session_id: Option<String>,

    /// The ID (filename prefix) of the parent report in the session
    #[clap(long, global = true, value_name = "REPORT_ID")]
    parent_report_id: Option<String>,

    /// The ID of the action selected from the parent report
    #[clap(long, global = true, value_name = "ACTION_ID")]
    selected_action_id: Option<String>,

    /// PostgreSQL connection string. Falls back to PG_LOGSTATS_DATABASE_URL
    /// and then [database].dsn from config.
    #[clap(long, global = true, value_name = "POSTGRES_URL")]
    dsn: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct SessionActionIndex {
    next_actions: Vec<NextAction>,
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
    since: CloudWatchSince,

    /// End time for CloudWatch input, as RFC3339. Defaults to now.
    #[clap(long, value_name = "TIME")]
    until: Option<CloudWatchUntil>,

    /// Optional CloudWatch Logs filter pattern
    #[clap(long, value_name = "PATTERN")]
    cloudwatch_filter_pattern: Option<String>,

    /// Maximum CloudWatch filter-log-events pages to read
    #[clap(long, value_name = "N", default_value_t = 20)]
    cloudwatch_max_pages: usize,

    /// AWS region for CloudWatch input. Requires --features aws-sdk.
    #[clap(long, value_name = "REGION")]
    aws_region: Option<String>,

    /// AWS profile for CloudWatch input. Requires --features aws-sdk.
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
    fn uses_cloudwatch(&self) -> bool {
        self.cloudwatch_log_group.is_some() || self.rds_instance.is_some()
    }

    fn cloudwatch_input(&self) -> CloudWatchInput {
        CloudWatchInput {
            log_group: self.cloudwatch_log_group.clone(),
            rds_instance: self.rds_instance.clone(),
            since: self.since.clone(),
            until: self.until.clone(),
            filter_pattern: self.cloudwatch_filter_pattern.clone(),
            max_pages: self.cloudwatch_max_pages,
            aws_region: self.aws_region.clone(),
            aws_profile: self.aws_profile.clone(),
            sample_size: self.sample_size,
        }
    }

    fn local_log_input(&self) -> LocalLogInput {
        LocalLogInput {
            log_dir: self.log_dir.clone(),
            sample_size: self.sample_size,
            logfile_list: self.logfile_list.clone(),
            log_files: self.log_files.clone(),
        }
    }
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Investigation-oriented top findings
    Top {
        #[clap(subcommand)]
        command: TopCommand,
    },
    /// Inspect the environment and determine the supported operating mode
    Inspect {
        #[clap(flatten)]
        input: LogInputArgs,
    },
    /// Slow-query investigation workflows
    SlowQueries {
        #[clap(subcommand)]
        command: SlowQueriesCommand,
    },
    /// Run a diagnostic SQL query against the database
    RunSql {
        /// Named parameter for the selected action, in NAME=VALUE form.
        #[clap(long = "parameter", value_name = "NAME=VALUE")]
        parameters: Vec<String>,
    },
    /// Monitor active database sessions
    #[clap(name = "running-queries")]
    RunningQueries,
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
enum InputFormat {
    /// Auto-detect among supported text formats.
    Auto,
    /// Local logs using the pg-logstats supported default text prefix.
    Default,
    /// Amazon RDS logs using `%t:%r:%u@%d:[%p]:`.
    Rds,
}

impl InputFormat {
    fn text_log_format(self) -> TextLogFormat {
        match self {
            Self::Auto => TextLogFormat::Auto,
            Self::Default => TextLogFormat::Default,
            Self::Rds => TextLogFormat::AwsRds,
        }
    }

    fn event_source_kind(self) -> EventSourceKind {
        match self {
            Self::Rds => EventSourceKind::AwsRds,
            Self::Auto | Self::Default => EventSourceKind::Stderr,
        }
    }
}

fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    let mut args = Arguments::parse();
    if args.session_id.is_none() {
        let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
        args.session_id = Some(timestamp);
    }
    let start_time = Instant::now();

    // Validate CLI arguments
    validate_arguments(&args)?;

    let resolved_config = load_config(args.workspace.as_deref())?;
    debug!("Loaded config from {:?}", resolved_config.source);
    let inspect_report = load_startup_inspect_report(&args)?;

    // Initialize parser based on format
    let parser = initialize_parser(&args)?;

    run_command(
        &args,
        &parser,
        &resolved_config.config,
        inspect_report.as_ref(),
    )?;

    let elapsed = start_time.elapsed();
    if !args.quiet {
        println!("Analysis completed in {:.2}s", elapsed.as_secs_f64());
    }

    Ok(())
}

fn validate_session_action(args: &Arguments, workspace_path: &Path) -> Result<()> {
    if (args.parent_report_id.is_some() || args.selected_action_id.is_some())
        && (args.parent_report_id.is_none() || args.selected_action_id.is_none())
    {
        return Err(PgLogstatsError::Configuration {
            message: "Both --parent-report-id and --selected-action-id must be specified together"
                .to_string(),
            field: Some("selected_action_id".to_string()),
        });
    }

    if let (Some(sess_id), Some(parent_id), Some(act_id)) = (
        &args.session_id,
        &args.parent_report_id,
        &args.selected_action_id,
    ) {
        let parent_path = workspace_path
            .join("sessions")
            .join(sess_id)
            .join("reports")
            .join(format!("{}.json", parent_id));

        if parent_path.exists() {
            let parent_content = fs::read_to_string(&parent_path)?;
            let parent_rep: SessionActionIndex =
                serde_json::from_str(&parent_content).map_err(PgLogstatsError::Serialization)?;

            let action = parent_rep
                .next_actions
                .iter()
                .find(|a| a.action_id == *act_id)
                .ok_or_else(|| PgLogstatsError::Configuration {
                    message: format!(
                        "Action ID '{}' not found in parent report next_actions",
                        act_id
                    ),
                    field: Some("selected_action_id".to_string()),
                })?;

            if action.status != NextActionStatus::Allowed {
                return Err(PgLogstatsError::Configuration {
                    message: format!(
                        "Action '{}' is not allowed in parent report. Status: {:?}, Reason: {}",
                        act_id, action.status, action.reason
                    ),
                    field: Some("selected_action_id".to_string()),
                });
            }
        } else {
            return Err(PgLogstatsError::Configuration {
                message: format!("Parent report '{}' not found in session", parent_id),
                field: Some("parent_report_id".to_string()),
            });
        }
    }
    Ok(())
}

fn run_command(
    args: &Arguments,
    parser: &TextLogParser,
    config: &pg_logstats::AppConfig,
    inspect_report: Option<&PgTriageReport<InspectReportPayload>>,
) -> Result<()> {
    let workspace = resolve_workspace_path(args.workspace.as_deref())?;
    validate_session_action(args, &workspace)?;

    match &args.command {
        Command::Top { .. } => run_top_query_families_command(args, parser, config, inspect_report),
        Command::Inspect { input } => {
            let cloudwatch_input = input.uses_cloudwatch().then(|| input.cloudwatch_input());
            let mut report = inspect(
                config,
                args.dsn.as_deref(),
                &input.local_log_input(),
                cloudwatch_input.as_ref(),
                parser,
                source_kind_for_input(args, input),
                args.session_id.clone(),
                args.workspace.as_deref(),
            )?;

            let workspace = resolve_workspace_path(args.workspace.as_deref())?;
            record_session_report(&workspace, &mut report, args)?;
            output_report(&report, args)?;
            Ok(())
        }
        Command::SlowQueries { .. } => {
            run_slow_queries_diff_command(args, parser, config, inspect_report)
        }
        Command::RunSql { .. } => run_sql_command(args, parser, config, inspect_report),
        Command::RunningQueries => {
            run_running_queries_command(args, parser, config, inspect_report)
        }
    }
}

fn load_default_log_entries(
    args: &Arguments,
    input: &LogInputArgs,
    parser: &TextLogParser,
) -> Result<Vec<pg_logstats::LogEntry>> {
    if input.uses_cloudwatch() {
        let entries = process_cloudwatch_input(&input.cloudwatch_input(), parser)?;
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
    let local_input = input.local_log_input();
    let log_files = discover_log_files(&local_input)?;

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
    parser: &TextLogParser,
    config: &pg_logstats::AppConfig,
    inspect_report: Option<&PgTriageReport<InspectReportPayload>>,
) -> Result<()> {
    let Command::Top {
        command: TopCommand::QueryFamilies { limit, input },
    } = &args.command
    else {
        unreachable!();
    };

    require_log_backed_mode(inspect_report)?;

    let all_entries = load_default_log_entries(args, input, parser)?;

    let findings =
        run_top_query_families(&all_entries, *limit, source_kind_for_input(args, input))?;
    let mut report =
        top_query_families_report(findings, &all_entries, source_kind_for_input(args, input));
    if let Some(ir) = inspect_report {
        report.operating_mode = ir.operating_mode;
    }
    report.session_id = args.session_id.clone();
    pg_logstats::populate_next_actions(&mut report, config);

    let workspace = resolve_workspace_path(args.workspace.as_deref())?;
    record_session_report(&workspace, &mut report, args)?;

    output_report(&report, args)
}

fn run_slow_queries_diff_command(
    args: &Arguments,
    parser: &TextLogParser,
    _config: &pg_logstats::AppConfig,
    inspect_report: Option<&PgTriageReport<InspectReportPayload>>,
) -> Result<()> {
    let Command::SlowQueries {
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
    } = &args.command
    else {
        unreachable!();
    };

    require_log_backed_mode(inspect_report)?;

    let options = SlowQueryDiffOptions {
        limit: *limit,
        min_target_count: *min_target_count,
        min_target_total_ms: *min_target_total_ms,
        min_p95_delta_ms: *min_p95_delta_ms,
    };

    let (findings, total_entries) = run_slow_queries_diff(
        baseline,
        target,
        parser,
        *sample_size,
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
        Command::Inspect { input, .. } => validate_log_input_args(input)?,
        Command::SlowQueries {
            command: SlowQueriesCommand::Diff { sample_size, .. },
        } => validate_sample_size(*sample_size)?,
        Command::RunSql { parameters } => {
            parse_action_parameters(parameters)?;
        }
        Command::RunningQueries => {}
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

    validate_file_input_args(&input.local_log_input())?;
    validate_sample_size(input.sample_size)
}

fn validate_cloudwatch_input_args(input: &LogInputArgs) -> Result<()> {
    if input.log_dir.is_some() || input.logfile_list.is_some() || !input.log_files.is_empty() {
        return Err(PgLogstatsError::Configuration {
            message: "CloudWatch input cannot be combined with local log files".to_string(),
            field: Some("cloudwatch_input".to_string()),
        });
    }

    pg_logstats::input::cloudwatch::validate_cloudwatch_input_args(&input.cloudwatch_input())
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

fn initialize_parser(args: &Arguments) -> Result<TextLogParser> {
    debug!("Initializing text log parser for {:?}", args.input_format);
    Ok(TextLogParser::with_format(
        args.input_format.text_log_format(),
    ))
}

fn source_kind_for_input(args: &Arguments, input: &LogInputArgs) -> EventSourceKind {
    if input.uses_cloudwatch() && matches!(args.input_format, InputFormat::Auto) {
        return EventSourceKind::AwsRds;
    }

    args.input_format.event_source_kind()
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
    parser: &TextLogParser,
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

fn record_session_report<T: serde::Serialize>(
    workspace_path: &Path,
    report: &mut PgTriageReport<T>,
    args: &Arguments,
) -> Result<()> {
    let sess_id = report
        .session_id
        .clone()
        .or_else(|| args.session_id.clone());

    let Some(sess_id_str) = sess_id else {
        return Ok(());
    };

    report.session_id = Some(sess_id_str.clone());

    let session_dir = workspace_path.join("sessions").join(&sess_id_str);
    let reports_dir = session_dir.join("reports");
    fs::create_dir_all(&reports_dir)?;

    if let (Some(parent_id), Some(act_id)) = (&args.parent_report_id, &args.selected_action_id) {
        let parent_path = reports_dir.join(format!("{}.json", parent_id));
        if parent_path.exists() {
            let parent_content = fs::read_to_string(&parent_path)?;
            let parent_rep: SessionActionIndex =
                serde_json::from_str(&parent_content).map_err(PgLogstatsError::Serialization)?;

            let action = parent_rep
                .next_actions
                .iter()
                .find(|a| a.action_id == *act_id)
                .ok_or_else(|| PgLogstatsError::Configuration {
                    message: format!(
                        "Action ID '{}' not found in parent report next_actions",
                        act_id
                    ),
                    field: Some("selected_action_id".to_string()),
                })?;

            if action.status != NextActionStatus::Allowed {
                return Err(PgLogstatsError::Configuration {
                    message: format!(
                        "Action '{}' is not allowed in parent report. Status: {:?}, Reason: {}",
                        act_id, action.status, action.reason
                    ),
                    field: Some("selected_action_id".to_string()),
                });
            }
        } else {
            return Err(PgLogstatsError::Configuration {
                message: format!("Parent report '{}' not found in session", parent_id),
                field: Some("parent_report_id".to_string()),
            });
        }
    }

    if let Some(parent_id) = &args.parent_report_id {
        report.parent_report_id = Some(parent_id.clone());
    }
    if let Some(act_id) = &args.selected_action_id {
        report.selected_action_id = Some(act_id.clone());
    }

    if report.created_at.is_none() {
        report.created_at = Some(chrono::Utc::now().to_rfc3339());
    }

    let sequence = fs::read_dir(&reports_dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .count()
        + 1;

    let report_id_str = format!("{:04}-{}", sequence, workflow_slug(report.workflow));
    report.report_id = Some(report_id_str.clone());

    let report_file = reports_dir.join(format!("{}.json", report_id_str));
    let content = serde_json::to_string_pretty(report).map_err(PgLogstatsError::Serialization)?;
    fs::write(report_file, content)?;

    Ok(())
}

fn load_startup_inspect_report(
    args: &Arguments,
) -> Result<Option<PgTriageReport<InspectReportPayload>>> {
    if matches!(args.command, Command::Inspect { .. }) {
        return Ok(None);
    }

    let workspace = resolve_workspace_path(args.workspace.as_deref())?;
    let path = workspace_inspect_report_path(&workspace);
    if !path.exists() {
        return Err(PgLogstatsError::Configuration {
            message: format!(
                "Inspect output not found at {}. Run `pg-logstats inspect` first.",
                path.display()
            ),
            field: Some("inspect_report".to_string()),
        });
    }

    let content = fs::read_to_string(&path)?;
    let report: PgTriageReport<InspectReportPayload> =
        serde_json::from_str(&content).map_err(PgLogstatsError::Serialization)?;

    if report.workflow != ActionKind::Inspect {
        return Err(PgLogstatsError::Configuration {
            message: format!(
                "Inspect output at {} is not an inspect report.",
                path.display()
            ),
            field: Some("inspect_report".to_string()),
        });
    }

    Ok(Some(report))
}

fn require_log_backed_mode(
    inspect_report: Option<&PgTriageReport<InspectReportPayload>>,
) -> Result<()> {
    let Some(report) = inspect_report else {
        return Err(PgLogstatsError::Configuration {
            message: "Inspect output is required before running this command.".to_string(),
            field: Some("inspect_report".to_string()),
        });
    };

    if report.operating_mode != OperatingMode::LogBackedAndLive
        && report.operating_mode != OperatingMode::LogBackedOnly
    {
        return Err(PgLogstatsError::Configuration {
            message: format!(
                "This command requires log-backed capability, but inspect reported {}.",
                match report.operating_mode {
                    OperatingMode::LogBackedAndLive => "log_backed_and_live",
                    OperatingMode::LogBackedOnly => "log_backed_only",
                    OperatingMode::LiveOnly => "live_only",
                    OperatingMode::Unready => "unready",
                }
            ),
            field: Some("operating_mode".to_string()),
        });
    }

    Ok(())
}

fn run_sql_command(
    args: &Arguments,
    _parser: &TextLogParser,
    config: &pg_logstats::AppConfig,
    inspect_report: Option<&PgTriageReport<InspectReportPayload>>,
) -> Result<()> {
    let Command::RunSql { parameters } = &args.command else {
        unreachable!();
    };

    let workspace = resolve_workspace_path(args.workspace.as_deref())?;
    let parsed_parameters: Vec<ActionParameterInput> = parse_action_parameters(parameters)?;
    let mut sql_report = execute_run_sql(
        &RunSqlRequest {
            workspace_path: &workspace,
            session_id: args.session_id.as_deref(),
            parent_report_id: args.parent_report_id.as_deref(),
            selected_action_id: args.selected_action_id.as_deref(),
            dsn: args.dsn.as_deref(),
            operating_mode: inspect_report
                .map(|report| report.operating_mode)
                .unwrap_or(OperatingMode::LiveOnly),
            parameters: &parsed_parameters,
        },
        config,
    )?;

    pg_logstats::populate_next_actions(&mut sql_report, config);

    record_session_report(&workspace, &mut sql_report, args)?;

    output_report(&sql_report, args)
}

fn run_running_queries_command(
    args: &Arguments,
    _parser: &TextLogParser,
    config: &pg_logstats::AppConfig,
    inspect_report: Option<&PgTriageReport<InspectReportPayload>>,
) -> Result<()> {
    let workspace = resolve_workspace_path(args.workspace.as_deref())?;
    let mut report = run_running_queries(
        args.dsn.as_deref(),
        config,
        inspect_report,
        args.session_id.clone(),
    )?;

    record_session_report(&workspace, &mut report, args)?;
    output_report(&report, args)
}

fn output_report<T: serde::Serialize>(report: &PgTriageReport<T>, args: &Arguments) -> Result<()> {
    pg_logstats::output_report(
        report,
        args.output_format,
        args.outfile.as_deref(),
        args.outdir.as_deref(),
    )
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
