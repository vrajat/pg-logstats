//! Output formatters and helpers for pg-logstats analysis results

pub mod json;
pub mod text;

pub use json::JsonFormatter;
pub use text::TextFormatter;

use crate::triage::{ActionKind, PgTriageReport};
use clap::ValueEnum;
use std::path::{Path, PathBuf};

/// Output formats supported by pg-logstats.
#[derive(Debug, ValueEnum, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Text format for human-readable console output.
    Text,
    /// JSON format for structured, machine-readable output.
    Json,
}

/// Helper function to write output string to a file (or outdir/outfile path) or print to stdout.
pub fn write_or_print_output(
    output: &str,
    outfile: Option<&str>,
    outdir: Option<&str>,
) -> Result<(), crate::PgLogstatsError> {
    if let Some(outfile) = outfile {
        if outfile == "-" {
            println!("{}", output);
        } else {
            let output_path = if let Some(outdir) = outdir {
                Path::new(outdir).join(outfile)
            } else {
                PathBuf::from(outfile)
            };
            std::fs::write(&output_path, output)?;
        }
    } else {
        println!("{}", output);
    }
    Ok(())
}

/// Formats and outputs a triage report in JSON or Text format to stdout or file.
pub fn output_report<T: serde::Serialize>(
    report: &PgTriageReport<T>,
    format: OutputFormat,
    outfile: Option<&str>,
    outdir: Option<&str>,
) -> Result<(), crate::PgLogstatsError> {
    match format {
        OutputFormat::Json => {
            let output = serde_json::to_string_pretty(report)
                .map_err(crate::PgLogstatsError::Serialization)?;
            write_or_print_output(&output, outfile, outdir)?;
        }
        OutputFormat::Text => match report.workflow {
            ActionKind::Inspect => {
                let json_val =
                    serde_json::to_value(report).map_err(crate::PgLogstatsError::Serialization)?;
                let inspect_report: PgTriageReport<crate::inspect::InspectReportPayload> =
                    serde_json::from_value(json_val)
                        .map_err(crate::PgLogstatsError::Serialization)?;
                let output = crate::inspect::format_inspect_text(&inspect_report);
                write_or_print_output(&output, outfile, outdir)?;
            }
            ActionKind::TopQueryFamilies => {
                let json_val =
                    serde_json::to_value(report).map_err(crate::PgLogstatsError::Serialization)?;
                let top_report: PgTriageReport<crate::findings::FindingsPayload> =
                    serde_json::from_value(json_val)
                        .map_err(crate::PgLogstatsError::Serialization)?;
                let findings = crate::findings::FindingSet::new(top_report.payload.findings);
                let formatter = TextFormatter::new();
                let output = formatter.format_findings(&findings)?;
                write_or_print_output(&output, outfile, outdir)?;
            }
            ActionKind::RunSql => {
                let json_val =
                    serde_json::to_value(report).map_err(crate::PgLogstatsError::Serialization)?;
                let sql_report: PgTriageReport<crate::triage::SqlActionPayload> =
                    serde_json::from_value(json_val)
                        .map_err(crate::PgLogstatsError::Serialization)?;

                let mut output = String::new();
                output.push_str(&format!("Action ID: {}\n", sql_report.payload.action_id));
                if let Some(source_finding_id) = &sql_report.payload.source_finding_id {
                    output.push_str(&format!("Source Finding: {}\n", source_finding_id));
                }
                if !sql_report.payload.insights.is_empty() {
                    output.push_str("Insights:\n");
                    for insight in &sql_report.payload.insights {
                        let confidence = match insight.confidence {
                            crate::triage::SqlInsightConfidence::High => "high",
                            crate::triage::SqlInsightConfidence::Medium => "medium",
                        };
                        output.push_str(&format!(
                            "- {} [{}]: {}\n",
                            insight.label, confidence, insight.reason
                        ));
                    }
                }
                output.push('\n');
                if sql_report.payload.row_count == 0 {
                    output.push_str("No rows returned.\n");
                } else {
                    output.push_str(&sql_report.payload.columns.join("\t"));
                    output.push('\n');
                    for r in &sql_report.payload.rows {
                        let row_strs: Vec<String> = r
                            .iter()
                            .map(|v| match v {
                                serde_json::Value::String(s) => s.clone(),
                                _ => v.to_string(),
                            })
                            .collect();
                        output.push_str(&row_strs.join("\t"));
                        output.push('\n');
                    }
                    if sql_report.payload.truncated {
                        output.push_str("(Results truncated to 20 rows)\n");
                    }
                }
                write_or_print_output(&output, outfile, outdir)?;
            }
            _ => {
                let output = serde_json::to_string_pretty(report)
                    .map_err(crate::PgLogstatsError::Serialization)?;
                write_or_print_output(&output, outfile, outdir)?;
            }
        },
    }
    Ok(())
}
