use clap::{Args, Parser, Subcommand, ValueEnum};
use indicatif::{ProgressBar, ProgressStyle};
use log::{debug, error, info, warn};
use pg_logstats::{
    error_class_findings, errors_report, execute_agent_install, execute_run_sql,
    input::{
        discover_log_files, process_cloudwatch_input, process_log_file, process_log_paths,
        validate_file_input_args, CloudWatchInput, CloudWatchSince, CloudWatchUntil, LocalLogInput,
    },
    inspect, load_config, normalize_log_entries, parse_action_parameters, query_family_findings,
    resolve_workspace_path, run_running_queries, slow_query_diff_findings, temp_file_findings,
    temp_files_report, top_query_families_report, workspace_inspect_report_path, ActionKind,
    ActionParameterInput, Correlator, EventSourceKind, InspectReportPayload, JsonFormatter,
    OperatingMode, OutputFormat, PgLogstatsError, PgTriageReport, ProcessOrderCorrelator,
    ReportStore, Result, RunSqlRequest, SlowQueryDiffOptions, TextFormatter, TextLogFormat,
    TextLogParser,
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
    #[clap(long, global = true, value_enum, default_value = "json")]
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

    /// Parent triage report used by a follow-up workflow. Accepts a report ID or path.
    #[clap(long, global = true, value_name = "REPORT")]
    triage_report: Option<String>,

    /// The ID of the action selected from the parent triage report
    #[clap(long, global = true, value_name = "ACTION_ID")]
    action_id: Option<String>,

    /// PostgreSQL connection string. Falls back to PG_LOGSTATS_DATABASE_URL
    /// and then [database].dsn from config.
    #[clap(long, global = true, value_name = "POSTGRES_URL")]
    dsn: Option<String>,
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
        command: Option<SlowQueriesCommand>,
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
    /// surface grouped error and event triage in a bounded historical window
    Errors {
        /// Maximum number of error-class findings to emit
        #[clap(long, default_value_t = 10)]
        limit: usize,

        #[clap(flatten)]
        input: LogInputArgs,
    },
    /// surface temp-file-driven resource pressure in a bounded historical window
    #[clap(name = "temp-files")]
    TempFiles {
        /// Maximum number of temp-file findings to emit
        #[clap(long, default_value_t = 10)]
        limit: usize,

        #[clap(flatten)]
        input: LogInputArgs,
    },
    /// AI agent guidance and playbooks commands
    Agent {
        #[clap(subcommand)]
        command: AgentCommand,
    },
}

#[derive(Debug, Subcommand)]
enum AgentCommand {
    /// Install agent guidance and playbook skills
    Install {
        /// Target AI agent harness (codex, claude, or gemini)
        #[clap(long, value_name = "HARNESS")]
        harness: String,

        /// Status check only, do not write or change any files
        #[clap(long)]
        status: bool,

        /// Dry run, print intended writes without modifying files
        #[clap(long)]
        dry_run: bool,
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

fn main() {
    if let Err(err) = try_main() {
        eprintln!("{err}");
        process::exit(1);
    }
}

fn try_main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    let args = Arguments::parse();
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

fn run_command(
    args: &Arguments,
    parser: &TextLogParser,
    config: &pg_logstats::AppConfig,
    inspect_report: Option<&PgTriageReport<InspectReportPayload>>,
) -> Result<()> {
    match &args.command {
        Command::Top { .. } => run_top_query_families_command(args, parser, config, inspect_report),
        Command::Inspect { input } => {
            let cloudwatch_input = input.uses_cloudwatch().then(|| input.cloudwatch_input());
            let report = inspect(
                config,
                args.dsn.as_deref(),
                &input.local_log_input(),
                cloudwatch_input.as_ref(),
                parser,
                source_kind_for_input(args, input),
                args.workspace.as_deref(),
            )?;

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
        Command::Errors { limit, input } => {
            run_errors_command(args, *limit, input, parser, config, inspect_report)
        }
        Command::TempFiles { limit, input } => {
            run_temp_files_command(args, *limit, input, parser, config, inspect_report)
        }
        Command::Agent { command } => run_agent_command(args, command, config),
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
    pg_logstats::populate_next_actions(&mut report, config);

    persist_workspace_report(args, &mut report)?;

    output_report(&report, args)
}

fn run_slow_queries_diff_command(
    args: &Arguments,
    parser: &TextLogParser,
    _config: &pg_logstats::AppConfig,
    inspect_report: Option<&PgTriageReport<InspectReportPayload>>,
) -> Result<()> {
    let Command::SlowQueries {
        command,
    } = &args.command
    else {
        unreachable!();
    };

    let Some(SlowQueriesCommand::Diff {
        baseline,
        target,
        sample_size,
        limit,
        min_target_count,
        min_target_total_ms,
        min_p95_delta_ms,
    }) = command
    else {
        require_log_backed_mode(inspect_report)?;

        return Err(PgLogstatsError::Configuration {
            message: concat!(
                "`pg-logstats slow-queries` is not the first slow-query triage step.\n",
                "Run `pg-logstats inspect --output-format json /path/to/postgresql.log` first.\n",
                "Then run `pg-logstats top query-families --output-format json /path/to/postgresql.log` for single-window slow-query triage.\n",
                "Use `pg-logstats slow-queries diff --baseline ... --target ...` only when you already have explicit baseline and target log windows."
            )
            .to_string(),
            field: Some("slow_queries".to_string()),
        });
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
            command: Some(SlowQueriesCommand::Diff { sample_size, .. }),
        } => validate_sample_size(*sample_size)?,
        Command::SlowQueries { command: None } => {}
        Command::RunSql { parameters } => {
            parse_action_parameters(parameters)?;
        }
        Command::RunningQueries => {}
        Command::Errors { input, .. } => validate_log_input_args(input)?,
        Command::TempFiles { input, .. } => validate_log_input_args(input)?,
        Command::Agent { .. } => {}
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

fn persist_workspace_report<T: serde::Serialize>(
    args: &Arguments,
    report: &mut PgTriageReport<T>,
) -> Result<PathBuf> {
    let workspace = resolve_workspace_path(args.workspace.as_deref())?;
    let store = ReportStore::new(&workspace);
    store.persist(report)
}

fn load_startup_inspect_report(
    args: &Arguments,
) -> Result<Option<PgTriageReport<InspectReportPayload>>> {
    if matches!(args.command, Command::Inspect { .. } | Command::Agent { .. }) {
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

fn require_ready_mode(
    inspect_report: Option<&PgTriageReport<InspectReportPayload>>,
) -> Result<()> {
    let Some(report) = inspect_report else {
        return Err(PgLogstatsError::Configuration {
            message: "Inspect output is required before running this command.".to_string(),
            field: Some("inspect_report".to_string()),
        });
    };

    if report.operating_mode == OperatingMode::Unready {
        return Err(PgLogstatsError::Configuration {
            message: "This command cannot run when inspect reported unready.".to_string(),
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

    require_ready_mode(inspect_report)?;

    let workspace = resolve_workspace_path(args.workspace.as_deref())?;
    let parsed_parameters: Vec<ActionParameterInput> = parse_action_parameters(parameters)?;
    let triage_report = args
        .triage_report
        .as_deref()
        .ok_or_else(|| PgLogstatsError::Configuration {
            message: "run-sql requires --triage-report".to_string(),
            field: Some("triage_report".to_string()),
        })?;
    let action_id = args.action_id.as_deref().ok_or_else(|| PgLogstatsError::Configuration {
        message: "run-sql requires --action-id".to_string(),
        field: Some("action_id".to_string()),
    })?;
    let parent_report = ReportStore::new(&workspace).load_report_base(triage_report)?;
    let mut sql_report = execute_run_sql(
        &RunSqlRequest {
            workspace_path: &workspace,
            triage_report,
            action_id,
            dsn: args.dsn.as_deref(),
            operating_mode: inspect_report
                .map(|report| report.operating_mode)
                .unwrap_or(OperatingMode::LiveOnly),
            parameters: &parsed_parameters,
        },
        config,
    )?;

    pg_logstats::populate_next_actions(&mut sql_report, config);
    sql_report.parent_report_id = parent_report.report_id;
    sql_report.selected_action_id = Some(action_id.to_string());
    persist_workspace_report(args, &mut sql_report)?;

    output_report(&sql_report, args)
}

fn run_running_queries_command(
    args: &Arguments,
    _parser: &TextLogParser,
    config: &pg_logstats::AppConfig,
    inspect_report: Option<&PgTriageReport<InspectReportPayload>>,
) -> Result<()> {
    require_ready_mode(inspect_report)?;

    let mut report = run_running_queries(args.dsn.as_deref(), config, inspect_report)?;
    persist_workspace_report(args, &mut report)?;
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

fn run_errors_command(
    args: &Arguments,
    limit: usize,
    input: &LogInputArgs,
    parser: &TextLogParser,
    config: &pg_logstats::AppConfig,
    inspect_report: Option<&PgTriageReport<InspectReportPayload>>,
) -> Result<()> {
    require_log_backed_mode(inspect_report)?;

    let all_entries = load_default_log_entries(args, input, parser)?;
    let source_kind = source_kind_for_input(args, input);
    let normalized_events = normalize_log_entries(&all_entries, source_kind);

    let findings = error_class_findings(&normalized_events, limit);
    let mut report = errors_report(findings, &all_entries, source_kind);

    if let Some(ir) = inspect_report {
        report.operating_mode = ir.operating_mode;
    }
    pg_logstats::populate_next_actions(&mut report, config);

    persist_workspace_report(args, &mut report)?;

    output_report(&report, args)
}

fn run_temp_files_command(
    args: &Arguments,
    limit: usize,
    input: &LogInputArgs,
    parser: &TextLogParser,
    config: &pg_logstats::AppConfig,
    inspect_report: Option<&PgTriageReport<InspectReportPayload>>,
) -> Result<()> {
    require_log_backed_mode(inspect_report)?;

    let all_entries = load_default_log_entries(args, input, parser)?;
    let source_kind = source_kind_for_input(args, input);
    let normalized_events = normalize_log_entries(&all_entries, source_kind);

    let has_temp_events = normalized_events
        .iter()
        .any(|e| pg_logstats::findings::parse_temp_file_message(e.message()).is_some());

    let mut log_temp_files_passed = false;
    if let Some(ir) = inspect_report {
        if let Some(check) = ir
            .payload
            .database_inspect
            .checks
            .get(&pg_logstats::InspectCheckId::LogTempFiles)
        {
            log_temp_files_passed = check.status == pg_logstats::CheckStatus::Passed;
        }
    }

    if !has_temp_events && !log_temp_files_passed {
        return Err(PgLogstatsError::Configuration {
            message: "The temp-files command requires `log_temp_files` to be enabled or temp file events present in logs.".to_string(),
            field: Some("log_temp_files".to_string()),
        });
    }

    let findings = temp_file_findings(&normalized_events, limit);

    let has_uncorrelated = findings.findings.iter().any(|f| {
        f.temp_file
            .as_ref()
            .is_some_and(|tf| tf.query_family_id.is_none())
    });

    let mut report = temp_files_report(findings, &all_entries, source_kind, has_uncorrelated);

    if let Some(ir) = inspect_report {
        report.operating_mode = ir.operating_mode;
    }
    pg_logstats::populate_next_actions(&mut report, config);

    persist_workspace_report(args, &mut report)?;

    output_report(&report, args)
}

fn run_agent_command(
    args: &Arguments,
    command: &AgentCommand,
    config: &pg_logstats::AppConfig,
) -> Result<()> {
    match command {
        AgentCommand::Install {
            harness,
            status,
            dry_run,
        } => {
            let report = execute_agent_install(harness, config, *dry_run, *status)?;
            output_report(&report, args)
        }
    }
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
