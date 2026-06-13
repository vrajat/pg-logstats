use crate::{
    config::workspace_reports_dir, workflow_slug, ActionKind, NextAction, PgLogstatsError,
    PgTriageReport, Result,
};
use chrono::Utc;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ReportStore {
    workspace_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct LoadedReportBase {
    pub path: PathBuf,
    pub workflow: ActionKind,
    pub report_id: Option<String>,
    pub next_actions: Vec<NextAction>,
    pub raw_content: String,
}

#[derive(Debug, Deserialize)]
struct ReportBase {
    workflow: ActionKind,
    report_id: Option<String>,
    next_actions: Vec<NextAction>,
}

impl ReportStore {
    pub fn new(workspace_path: impl AsRef<Path>) -> Self {
        Self {
            workspace_path: workspace_path.as_ref().to_path_buf(),
        }
    }

    pub fn reports_dir(&self) -> PathBuf {
        workspace_reports_dir(&self.workspace_path)
    }

    pub fn persist<T: Serialize>(&self, report: &mut PgTriageReport<T>) -> Result<PathBuf> {
        let reports_dir = self.reports_dir();
        fs::create_dir_all(&reports_dir)?;

        if report.created_at.is_none() {
            report.created_at = Some(Utc::now().to_rfc3339());
        }

        let report_id = match &report.report_id {
            Some(existing) => existing.clone(),
            None => self.generate_report_id(report.workflow)?,
        };
        report.report_id = Some(report_id.clone());

        let report_path = reports_dir.join(format!("{report_id}.json"));
        if report_path.exists() {
            return Err(PgLogstatsError::Configuration {
                message: format!(
                    "Refusing to overwrite existing triage report at {}",
                    report_path.display()
                ),
                field: Some("report_id".to_string()),
            });
        }

        let content = serde_json::to_string_pretty(report).map_err(PgLogstatsError::Serialization)?;
        fs::write(&report_path, content)?;

        Ok(report_path)
    }

    pub fn resolve_reference(&self, reference: &str) -> Result<PathBuf> {
        let raw_path = Path::new(reference);
        let candidate = if raw_path.is_absolute() || raw_path.components().count() > 1 {
            raw_path.to_path_buf()
        } else if raw_path.extension().is_some() {
            self.reports_dir().join(raw_path)
        } else {
            self.reports_dir().join(format!("{reference}.json"))
        };

        if !candidate.exists() {
            return Err(PgLogstatsError::Configuration {
                message: format!(
                    "Triage report '{}' not found. Checked {}.",
                    reference,
                    candidate.display()
                ),
                field: Some("triage_report".to_string()),
            });
        }

        if !candidate.is_file() {
            return Err(PgLogstatsError::Configuration {
                message: format!("Triage report path is not a file: {}", candidate.display()),
                field: Some("triage_report".to_string()),
            });
        }

        Ok(candidate)
    }

    pub fn load_report_base(&self, reference: &str) -> Result<LoadedReportBase> {
        let path = self.resolve_reference(reference)?;
        let raw_content = fs::read_to_string(&path)?;
        let parsed: ReportBase =
            serde_json::from_str(&raw_content).map_err(PgLogstatsError::Serialization)?;
        let report_id = parsed
            .report_id
            .clone()
            .or_else(|| path.file_stem().map(|stem| stem.to_string_lossy().into_owned()));

        Ok(LoadedReportBase {
            path,
            workflow: parsed.workflow,
            report_id,
            next_actions: parsed.next_actions,
            raw_content,
        })
    }

    pub fn load_report<T: DeserializeOwned>(&self, reference: &str) -> Result<PgTriageReport<T>> {
        let path = self.resolve_reference(reference)?;
        let content = fs::read_to_string(path)?;
        serde_json::from_str(&content).map_err(PgLogstatsError::Serialization)
    }

    fn generate_report_id(&self, workflow: ActionKind) -> Result<String> {
        let timestamp = Utc::now().format("%Y%m%dT%H%M%S%6fZ").to_string();
        let slug = workflow_slug(workflow);
        let reports_dir = self.reports_dir();
        let mut report_id = format!("{timestamp}-{slug}");
        let mut report_path = reports_dir.join(format!("{report_id}.json"));
        let mut suffix = 2usize;

        while report_path.exists() {
            report_id = format!("{timestamp}-{slug}-{suffix}");
            report_path = reports_dir.join(format!("{report_id}.json"));
            suffix += 1;
        }

        Ok(report_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ActionKind, FindingsPayload, OperatingMode, PgTriageReport, PG_TRIAGE_SCHEMA_VERSION,
    };
    use tempfile::TempDir;

    fn sample_report() -> PgTriageReport<FindingsPayload> {
        PgTriageReport {
            schema_version: PG_TRIAGE_SCHEMA_VERSION,
            workflow: ActionKind::TopQueryFamilies,
            operating_mode: OperatingMode::LogBackedOnly,
            limitations: Vec::new(),
            verdict: None,
            verdict_reasons: Vec::new(),
            allowed_actions: None,
            blocked_actions: None,
            analysis_window: None,
            source_summary: None,
            next_actions: Vec::new(),
            report_id: None,
            parent_report_id: None,
            selected_action_id: None,
            created_at: None,
            payload: FindingsPayload { findings: Vec::new() },
        }
    }

    #[test]
    fn persist_writes_immutable_reports_under_workspace_root() {
        let temp_dir = TempDir::new().unwrap();
        let store = ReportStore::new(temp_dir.path());
        let mut report = sample_report();

        let path = store.persist(&mut report).unwrap();

        assert!(path.starts_with(temp_dir.path().join("reports")));
        assert!(path.exists());
        assert!(report.report_id.is_some());
        assert!(report.created_at.is_some());
    }

    #[test]
    fn resolve_reference_supports_report_ids() {
        let temp_dir = TempDir::new().unwrap();
        let store = ReportStore::new(temp_dir.path());
        let mut report = sample_report();

        let path = store.persist(&mut report).unwrap();
        let resolved = store
            .resolve_reference(report.report_id.as_deref().unwrap())
            .unwrap();

        assert_eq!(resolved, path);
    }

    #[test]
    fn persist_never_overwrites_previous_reports() {
        let temp_dir = TempDir::new().unwrap();
        let store = ReportStore::new(temp_dir.path());
        let mut first = sample_report();
        let mut second = sample_report();

        let first_path = store.persist(&mut first).unwrap();
        let second_path = store.persist(&mut second).unwrap();

        assert_ne!(first_path, second_path);
    }
}
