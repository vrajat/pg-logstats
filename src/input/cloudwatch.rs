use crate::{LogEntry, PgLogstatsError, Result, TextLogParser};
use chrono::Utc;
use std::fs;

#[derive(Debug, Clone)]
pub struct CloudWatchInput {
    pub log_group: Option<String>,
    pub rds_instance: Option<String>,
    pub since: String,
    pub until: Option<String>,
    pub filter_pattern: Option<String>,
    pub max_pages: usize,
    pub aws_region: Option<String>,
    pub aws_profile: Option<String>,
    pub sample_size: Option<usize>,
}

impl CloudWatchInput {
    pub fn log_group_name(&self) -> Option<String> {
        if let Some(log_group) = &self.log_group {
            return Some(log_group.clone());
        }

        self.rds_instance
            .as_ref()
            .map(|instance| format!("/aws/rds/instance/{instance}/postgresql"))
    }
}

pub fn validate_cloudwatch_input_args(input: &CloudWatchInput) -> Result<()> {
    if input.max_pages == 0 {
        return Err(PgLogstatsError::Configuration {
            message: "CloudWatch max pages must be greater than 0".to_string(),
            field: Some("cloudwatch_max_pages".to_string()),
        });
    }

    Ok(())
}

pub fn process_cloudwatch_input(
    input: &CloudWatchInput,
    parser: &TextLogParser,
) -> Result<Vec<LogEntry>> {
    let log_group = input
        .log_group_name()
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
    input: &CloudWatchInput,
    log_group: &str,
) -> Result<Vec<CloudWatchLogEvent>> {
    if let Some(fixture_response) = read_cloudwatch_fixture_response()? {
        return Ok(fixture_response.events);
    }

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

    for _page in 0..input.max_pages {
        let response = filter_cloudwatch_log_events(
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

fn read_cloudwatch_fixture_response() -> Result<Option<CloudWatchFilterEventsResponse>> {
    let Some(path) = std::env::var_os("PG_LOGSTATS_CLOUDWATCH_FIXTURE") else {
        return Ok(None);
    };

    let content = fs::read(path)?;
    serde_json::from_slice(&content)
        .map(Some)
        .map_err(PgLogstatsError::Serialization)
}

#[cfg(feature = "aws-sdk")]
fn filter_cloudwatch_log_events(
    input: &CloudWatchInput,
    log_group: &str,
    start_time: i64,
    end_time: i64,
    next_token: Option<&str>,
) -> Result<CloudWatchFilterEventsResponse> {
    let runtime = tokio::runtime::Runtime::new().map_err(|err| PgLogstatsError::Unexpected {
        message: format!("Failed to create async runtime for CloudWatch input: {err}"),
        context: Some("cloudwatch".to_string()),
    })?;

    runtime.block_on(filter_cloudwatch_log_events_async(
        input, log_group, start_time, end_time, next_token,
    ))
}

#[cfg(feature = "aws-sdk")]
async fn filter_cloudwatch_log_events_async(
    input: &CloudWatchInput,
    log_group: &str,
    start_time: i64,
    end_time: i64,
    next_token: Option<&str>,
) -> Result<CloudWatchFilterEventsResponse> {
    use aws_config::BehaviorVersion;
    use aws_sdk_cloudwatchlogs::config::Region;

    let mut config_loader = aws_config::defaults(BehaviorVersion::latest());
    if let Some(region) = &input.aws_region {
        config_loader = config_loader.region(Region::new(region.clone()));
    }
    if let Some(profile) = &input.aws_profile {
        config_loader = config_loader.profile_name(profile);
    }

    let config = config_loader.load().await;
    let client = aws_sdk_cloudwatchlogs::Client::new(&config);
    let mut request = client
        .filter_log_events()
        .log_group_name(log_group)
        .start_time(start_time)
        .end_time(end_time);

    if let Some(filter_pattern) = &input.filter_pattern {
        request = request.filter_pattern(filter_pattern);
    }
    if let Some(next_token) = next_token {
        request = request.next_token(next_token);
    }

    let output = request
        .send()
        .await
        .map_err(|err| PgLogstatsError::Configuration {
            message: format!("CloudWatch filter-log-events failed: {err}"),
            field: Some("cloudwatch".to_string()),
        })?;

    let events = output
        .events()
        .iter()
        .map(|event| CloudWatchLogEvent {
            message: event.message().map(str::to_string),
        })
        .collect();

    Ok(CloudWatchFilterEventsResponse {
        events,
        next_token: output.next_token().map(str::to_string),
    })
}

#[cfg(not(feature = "aws-sdk"))]
fn filter_cloudwatch_log_events(
    _input: &CloudWatchInput,
    _log_group: &str,
    _start_time: i64,
    _end_time: i64,
    _next_token: Option<&str>,
) -> Result<CloudWatchFilterEventsResponse> {
    Err(PgLogstatsError::Configuration {
        message: "CloudWatch input requires building pg-logstats with `--features aws-sdk`"
            .to_string(),
        field: Some("cloudwatch".to_string()),
    })
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
