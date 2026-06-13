//! Agent guidance installation logic for Phase 8.

use crate::triage::{ActionKind, OperatingMode, PgTriageReport, PG_TRIAGE_SCHEMA_VERSION};
use crate::{PgLogstatsError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The payload returned by the `agent install` command.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentInstallPayload {
    /// The target agent harness (codex, claude, or gemini).
    pub harness: String,
    /// The path where the main configuration or skill file was written.
    pub install_location: String,
    /// Files that were newly created/written.
    pub files_written: Vec<String>,
    /// Files that were updated with new content.
    pub files_updated: Vec<String>,
    /// Status of the installation (e.g., "installed", "missing", "dry_run").
    pub status: String,
}

/// Resolves the absolute path to the main command/skill file and the playbook directory.
pub fn resolve_agent_paths(harness: &str, config: &crate::AppConfig) -> Result<(PathBuf, PathBuf)> {
    let home = std::env::var_os("HOME").map(PathBuf::from).ok_or_else(|| {
        PgLogstatsError::Configuration {
            message: "HOME environment variable not set".to_string(),
            field: None,
        }
    })?;

    match harness {
        "codex" => {
            let agents_md = config
                .agent_install
                .codex
                .agents_md_path
                .clone()
                .unwrap_or_else(|| home.join("AGENTS.md"));
            let playbook_dir = config
                .agent_install
                .codex
                .playbook_dir
                .clone()
                .unwrap_or_else(|| home.join(".config/pg-logstats/agents"));
            Ok((agents_md, playbook_dir))
        }
        "claude" => {
            let skill_dir = config
                .agent_install
                .claude
                .skill_dir
                .clone()
                .unwrap_or_else(|| home.join(".claude/skills"));
            let main_file = skill_dir.join("pg-logstats-triage").join("SKILL.md");
            let playbook_dir = skill_dir.join("pg-logstats-triage");
            Ok((main_file, playbook_dir))
        }
        "gemini" => {
            let commands_dir = config
                .agent_install
                .gemini
                .commands_dir
                .clone()
                .unwrap_or_else(|| home.join(".gemini/commands"));
            let main_file = commands_dir.join("pg-logstats-triage.toml");
            let playbook_dir = commands_dir.join("pg-logstats-triage");
            Ok((main_file, playbook_dir))
        }
        _ => Err(PgLogstatsError::Configuration {
            message: format!("Unsupported harness: {}", harness),
            field: Some("harness".to_string()),
        }),
    }
}

/// Checks if the agent installation exists and is verified.
pub fn check_agent_status(harness: &str, config: &crate::AppConfig) -> Result<bool> {
    let home = std::env::var_os("HOME").map(PathBuf::from).ok_or_else(|| {
        PgLogstatsError::Configuration {
            message: "HOME environment variable not set".to_string(),
            field: None,
        }
    })?;

    match harness {
        "codex" => {
            let path = config
                .agent_install
                .codex
                .agents_md_path
                .clone()
                .unwrap_or_else(|| home.join("AGENTS.md"));
            if !path.exists() {
                return Ok(false);
            }
            let content = std::fs::read_to_string(path)?;
            Ok(content.contains("<!-- START pg-logstats agent guidance -->"))
        }
        "claude" => {
            let skill_dir = config
                .agent_install
                .claude
                .skill_dir
                .clone()
                .unwrap_or_else(|| home.join(".claude/skills"));
            let path = skill_dir.join("pg-logstats-triage").join("SKILL.md");
            Ok(path.exists())
        }
        "gemini" => {
            let commands_dir = config
                .agent_install
                .gemini
                .commands_dir
                .clone()
                .unwrap_or_else(|| home.join(".gemini/commands"));
            let path = commands_dir.join("pg-logstats-triage.toml");
            Ok(path.exists())
        }
        _ => Err(PgLogstatsError::Configuration {
            message: format!("Unsupported harness: {}", harness),
            field: Some("harness".to_string()),
        }),
    }
}

/// Updates the AGENTS.md file for Codex in an idempotent way using markers.
fn update_codex_agents_md(
    path: &Path,
    playbook_path: &Path,
    dry_run: bool,
) -> Result<(bool, bool)> {
    let playbook_path_str = playbook_path.display().to_string();
    let content_to_insert = format!(
        "<!-- START pg-logstats agent guidance -->\n\
        ## pg-logstats Integration\n\n\
        You have `pg-logstats` available to perform PostgreSQL triage. Always use the following rule:\n\
        - Refer to the shared pg-logstats playbook located at: {}\n\
        - Always run `pg-logstats inspect --output-format json` before attempting other diagnostic actions.\n\
        - Respect `operating_mode`, `verdict`, and only run actions listed in `next_actions[]`.\n\
        <!-- END pg-logstats agent guidance -->",
        playbook_path_str
    );

    if !path.exists() {
        if !dry_run {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, format!("# AGENTS.md\n\n{}\n", content_to_insert))?;
        }
        return Ok((true, false));
    }

    let existing_content = std::fs::read_to_string(path)?;
    let start_marker = "<!-- START pg-logstats agent guidance -->";
    let end_marker = "<!-- END pg-logstats agent guidance -->";

    if let (Some(start_idx), Some(end_idx)) = (
        existing_content.find(start_marker),
        existing_content.find(end_marker),
    ) {
        let before = &existing_content[..start_idx];
        let after = &existing_content[end_idx + end_marker.len()..];
        let new_content = format!("{}{}{}", before, content_to_insert, after);

        if existing_content == new_content {
            return Ok((false, false));
        }

        if !dry_run {
            std::fs::write(path, new_content)?;
        }
        Ok((false, true))
    } else {
        let mut new_content = existing_content.clone();
        if !new_content.ends_with('\n') {
            new_content.push('\n');
        }
        new_content.push('\n');
        new_content.push_str(&content_to_insert);
        new_content.push('\n');

        if !dry_run {
            std::fs::write(path, new_content)?;
        }
        Ok((false, true))
    }
}

/// Generic helper to write or update a file.
fn write_or_update_file(path: &Path, content: &str, dry_run: bool) -> Result<(bool, bool)> {
    if !path.exists() {
        if !dry_run {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, content)?;
        }
        return Ok((true, false));
    }

    let existing = std::fs::read_to_string(path)?;
    if existing == content {
        return Ok((false, false));
    }

    if !dry_run {
        std::fs::write(path, content)?;
    }
    Ok((false, true))
}

/// The shared playbook content text.
fn playbook_content() -> &'static str {
    "# pg-logstats Triage Playbook\n\n\
     This playbook outlines the standard operating procedure for troubleshooting PostgreSQL database incidents using `pg-logstats`.\n\n\
     ## Core Workflow\n\n\
     Always follow these steps:\n\n\
     1. **Inspect First**: Run `pg-logstats inspect --output-format json` to detect the `operating_mode` and check the environment.\n\
     2. **Respect Operating Mode**:\n\
        - `log_backed_and_live` or `log_backed_only`: Safe to run log-based subcommands like `top query-families`, `errors`, and `temp-files`.\n\
        - `live_only`: Do NOT attempt log-based subcommands. Only run live checks like `running-queries`.\n\
        - `unready`: Triage setup is incomplete. Do not proceed; check config or database availability.\n\
     3. **DAG-driven Investigation**: Select and run commands ONLY from the `next_actions[]` array returned in the previous step's triage report JSON.\n\
     4. **Follow Verdict Policies**:\n\
        - `clear`: All read-only and stats actions are allowed.\n\
        - `busy`: Only narrow, low-impact stats actions are allowed.\n\
        - `saturated`: All actions are blocked. Stop adding database load and escalate.\n\
        - `unknown`: Do not infer safety; escalate or get better evidence.\n\
     5. **Stop and Escalate**: Stop diagnostic execution and notify a human operator when the database is `saturated`, the operating mode is `unready`, or the required action status is blocked.\n"
}

/// Executes the agent installation.
pub fn execute_agent_install(
    harness: &str,
    config: &crate::AppConfig,
    dry_run: bool,
    status_only: bool,
) -> Result<PgTriageReport<AgentInstallPayload>> {
    let (main_path, playbook_dir) = resolve_agent_paths(harness, config)?;
    let playbook_path = playbook_dir.join("playbook.md");

    let mut files_written = Vec::new();
    let mut files_updated = Vec::new();

    if status_only {
        let is_installed = check_agent_status(harness, config)?;
        let status = if is_installed { "installed" } else { "missing" };
        return Ok(PgTriageReport {
            schema_version: PG_TRIAGE_SCHEMA_VERSION,
            workflow: ActionKind::AgentInstall,
            operating_mode: OperatingMode::Unready, // Independent of database mode
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
            created_at: Some(chrono::Utc::now().to_rfc3339()),
            payload: AgentInstallPayload {
                harness: harness.to_string(),
                install_location: main_path.display().to_string(),
                files_written,
                files_updated,
                status: status.to_string(),
            },
        });
    }

    // Write playbook
    let (p_written, p_updated) = write_or_update_file(&playbook_path, playbook_content(), dry_run)?;
    if p_written {
        files_written.push(playbook_path.display().to_string());
    } else if p_updated {
        files_updated.push(playbook_path.display().to_string());
    }

    // Write main config/skill/command wrapper
    match harness {
        "codex" => {
            let (m_written, m_updated) =
                update_codex_agents_md(&main_path, &playbook_path, dry_run)?;
            if m_written {
                files_written.push(main_path.display().to_string());
            } else if m_updated {
                files_updated.push(main_path.display().to_string());
            }
        }
        "claude" => {
            let content = format!(
                "# pg-logstats Triage Skill\n\n\
                 This skill allows Claude to perform database triage using the `pg-logstats` CLI tool.\n\n\
                 ## Guidelines\n\n\
                 - Always run `pg-logstats inspect --output-format json` first.\n\
                 - Refer to the pg-logstats playbook at `{}` for full safety and operating instructions.\n\
                 - Choose follow-up actions from `next_actions[]` in the triage reports.\n\
                 - Stop and escalate if the database is saturated or operating mode is unready.\n",
                playbook_path.display()
            );
            let (m_written, m_updated) = write_or_update_file(&main_path, &content, dry_run)?;
            if m_written {
                files_written.push(main_path.display().to_string());
            } else if m_updated {
                files_updated.push(main_path.display().to_string());
            }
        }
        "gemini" => {
            let content = format!(
                "[command]\n\
                 name = \"pg-logstats-triage\"\n\
                 description = \"Troubleshoot PostgreSQL incidents using pg-logstats\"\n\n\
                 [instructions]\n\
                 playbook = \"{}\"\n\
                 prompt = \"\"\"\n\
                 You are a Gemini assistant equipped with pg-logstats.\n\
                 When troubleshooting:\n\
                 1. Always start by running `pg-logstats inspect --output-format json`.\n\
                 2. Review the pg-logstats playbook at '{}' for guidance.\n\
                 3. Only run commands suggested in `next_actions[]` from the triage report.\n\
                 4. Stop and escalate immediately if the database is saturated or operating mode is unready.\n\
                 \"\"\"\n",
                playbook_path.display(),
                playbook_path.display()
            );
            let (m_written, m_updated) = write_or_update_file(&main_path, &content, dry_run)?;
            if m_written {
                files_written.push(main_path.display().to_string());
            } else if m_updated {
                files_updated.push(main_path.display().to_string());
            }
        }
        _ => unreachable!(),
    }

    let status = if dry_run { "dry_run" } else { "installed" };

    Ok(PgTriageReport {
        schema_version: PG_TRIAGE_SCHEMA_VERSION,
        workflow: ActionKind::AgentInstall,
        operating_mode: OperatingMode::Unready,
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
        created_at: Some(chrono::Utc::now().to_rfc3339()),
        payload: AgentInstallPayload {
            harness: harness.to_string(),
            install_location: main_path.display().to_string(),
            files_written,
            files_updated,
            status: status.to_string(),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppConfig;
    use tempfile::TempDir;

    #[test]
    fn test_resolve_paths_and_status() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = AppConfig::default();
        config.agent_install.codex.agents_md_path = Some(temp_dir.path().join("AGENTS.md"));
        config.agent_install.codex.playbook_dir = Some(temp_dir.path().join("playbook_dir"));

        let (main_path, playbook_dir) = resolve_agent_paths("codex", &config).unwrap();
        assert_eq!(main_path, temp_dir.path().join("AGENTS.md"));
        assert_eq!(playbook_dir, temp_dir.path().join("playbook_dir"));

        // Status is false initially
        let is_installed = check_agent_status("codex", &config).unwrap();
        assert!(!is_installed);

        // Dry-run install
        let report = execute_agent_install("codex", &config, true, false).unwrap();
        assert_eq!(report.payload.status, "dry_run");
        assert_eq!(report.payload.files_written.len(), 2);
        assert!(!temp_dir.path().join("AGENTS.md").exists());

        // Actual install
        let report = execute_agent_install("codex", &config, false, false).unwrap();
        assert_eq!(report.payload.status, "installed");
        assert!(temp_dir.path().join("AGENTS.md").exists());
        assert!(temp_dir
            .path()
            .join("playbook_dir")
            .join("playbook.md")
            .exists());

        // Status is true now
        let is_installed = check_agent_status("codex", &config).unwrap();
        assert!(is_installed);

        // Idempotency: re-running does not change files
        let report = execute_agent_install("codex", &config, false, false).unwrap();
        assert!(report.payload.files_written.is_empty());
        assert!(report.payload.files_updated.is_empty());
    }
}
