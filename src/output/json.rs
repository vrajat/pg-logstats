//! JSON output formatter for pg-loggrep results

use crate::{AnalysisResult, TimingAnalysis, PgLoggrepError, Result};
use chrono::Utc;
use serde_json::json;
use std::collections::HashMap;

/// JSON formatter for analysis results
pub struct JsonFormatter {
	// Configuration for JSON formatting
	pretty: bool,
	tool_version: String,
	log_files_processed: Vec<String>,
	total_log_entries: usize,
}

impl JsonFormatter {
	/// Create a new JSON formatter
	pub fn new() -> Self {
		Self {
			pretty: false,
			tool_version: env!("CARGO_PKG_VERSION").to_string(),
			log_files_processed: Vec::new(),
			total_log_entries: 0,
		}
	}

	/// Enable or disable pretty printing
	pub fn with_pretty(mut self, pretty: bool) -> Self {
		self.pretty = pretty;
		self
	}

	/// Set metadata values for output
	pub fn with_metadata(
		mut self,
		tool_version: impl Into<String>,
		log_files_processed: Vec<String>,
		total_log_entries: usize,
	) -> Self {
		self.tool_version = tool_version.into();
		self.log_files_processed = log_files_processed;
		self.total_log_entries = total_log_entries;
		self
	}

	fn metadata_object(&self) -> serde_json::Value {
		json!({
			"analysis_timestamp": Utc::now().to_rfc3339(),
			"tool_version": self.tool_version,
			"log_files_processed": self.log_files_processed,
			"total_log_entries": self.total_log_entries,
		})
	}

	/// Format a single AnalysisResult as structured JSON
	pub fn format(&self, analysis: &AnalysisResult) -> Result<String> {
		let summary = json!({
			"total_queries": analysis.total_queries,
			"total_duration_ms": analysis.total_duration,
			"avg_duration_ms": analysis.average_duration,
			"error_count": analysis.error_count,
			"connection_count": analysis.connection_count,
		});

		let by_type = serde_json::to_value(&analysis.query_types)
			.map_err(PgLoggrepError::Serialization)?;

		// Build a map from query -> count to enrich slowest queries
		let mut freq_map: HashMap<String, u64> = HashMap::new();
		for (q, c) in &analysis.most_frequent_queries {
			freq_map.insert(q.clone(), *c);
		}

		let slowest_queries = analysis
			.slowest_queries
			.iter()
			.map(|(q, d)| {
				json!({
					"query": q,
					"duration_ms": d,
					"count": freq_map.get(q).cloned().unwrap_or(1),
				})
			})
			.collect::<Vec<_>>();

		let most_frequent = analysis
			.most_frequent_queries
			.iter()
			.map(|(q, c)| {
				json!({
					"query": q,
					"count": c,
					// Without per-query duration distribution, fall back to overall average
					"avg_duration_ms": analysis.average_duration,
				})
			})
			.collect::<Vec<_>>();

		let root = json!({
			"metadata": self.metadata_object(),
			"summary": summary,
			"query_analysis": {
				"by_type": by_type,
				"slowest_queries": slowest_queries,
				"most_frequent": most_frequent,
			},
		});

		if self.pretty {
			serde_json::to_string_pretty(&root).map_err(PgLoggrepError::Serialization)
		} else {
			serde_json::to_string(&root).map_err(PgLoggrepError::Serialization)
		}
	}

	/// Format with timing analysis included
	pub fn format_with_timing(&self, analysis: &AnalysisResult, timing: &TimingAnalysis) -> Result<String> {
		let mut base: serde_json::Value = serde_json::from_str(&self.format(analysis)?)
			.map_err(PgLoggrepError::Serialization)?;

		// Build temporal analysis section from TimingAnalysis
		let hourly_stats = timing
			.hourly_patterns
			.iter()
			.map(|(hour, total_ms)| {
				json!({
					"hour": hour,
					"total_duration_ms": total_ms,
				})
			})
			.collect::<Vec<_>>();

		let temporal = json!({
			"hourly_stats": hourly_stats,
			"average_response_time_ms": timing.average_response_time.num_milliseconds(),
			"p95_response_time_ms": timing.p95_response_time.num_milliseconds(),
			"p99_response_time_ms": timing.p99_response_time.num_milliseconds(),
		});

		if let Some(obj) = base.as_object_mut() {
			obj.insert("temporal_analysis".to_string(), temporal);
		}

		if self.pretty {
			serde_json::to_string_pretty(&base).map_err(PgLoggrepError::Serialization)
		} else {
			serde_json::to_string(&base).map_err(PgLoggrepError::Serialization)
		}
	}
}

impl Default for JsonFormatter {
	fn default() -> Self {
		Self::new()
	}
}
