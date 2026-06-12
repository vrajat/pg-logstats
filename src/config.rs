use crate::{config_error, guidance::RuleId, triage::RiskLabel, PgLogstatsError, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::{Path, PathBuf};

const WORKSPACE_ENV_VAR: &str = "PG_LOGSTATS_WORKSPACE";
const DEFAULT_WORKSPACE_RELATIVE_PATH: &str = ".local/share/pg-logstats";
const CONFIG_FILE_NAME: &str = "config.toml";
const INSPECT_FILE_NAME: &str = "inspect.json";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub running_queries: RunningQueriesConfig,
    pub guidance: GuidanceConfig,
    pub agent_install: AgentInstallConfig,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DatabaseConfig {
    pub dsn: Option<String>,
    pub connect_timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RunningQueriesConfig {
    pub thresholds: RunningQueriesThresholds,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RunningQueriesThresholds {
    pub long_running_query_ms: u64,
    pub waiting_session_count_threshold: u64,
    pub idle_in_transaction_count_threshold: u64,
}

impl Default for RunningQueriesThresholds {
    fn default() -> Self {
        Self {
            long_running_query_ms: 120_000,
            waiting_session_count_threshold: 2,
            idle_in_transaction_count_threshold: 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GuidanceConfig {
    pub max_risk: RiskLabel,
    pub show_omitted: bool,
    pub disabled_rules: Vec<RuleId>,
    #[serde(default)]
    pub rules: std::collections::HashMap<RuleId, RuleConfig>,
}

impl Default for GuidanceConfig {
    fn default() -> Self {
        Self {
            max_risk: RiskLabel::Bounded,
            show_omitted: true,
            disabled_rules: Vec::new(),
            rules: std::collections::HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RuleConfig {
    pub enabled: bool,
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AgentInstallConfig {
    pub active_harness: Option<String>,
    pub codex: AgentInstallTargetConfig,
    pub claude: AgentInstallTargetConfig,
    pub gemini: AgentInstallTargetConfig,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AgentInstallTargetConfig {
    pub agents_md_path: Option<PathBuf>,
    pub playbook_dir: Option<PathBuf>,
    pub skill_dir: Option<PathBuf>,
    pub commands_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigSource {
    BuiltInDefaults,
    ExplicitWorkspace(PathBuf),
    EnvWorkspace(PathBuf),
    DefaultWorkspace(PathBuf),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedConfig {
    pub config: AppConfig,
    pub source: ConfigSource,
}

pub fn workspace_env_var_name() -> &'static str {
    WORKSPACE_ENV_VAR
}

pub fn default_workspace_path() -> Option<PathBuf> {
    env::var_os("HOME").map(|home| PathBuf::from(home).join(DEFAULT_WORKSPACE_RELATIVE_PATH))
}

pub fn resolve_workspace_path(explicit_path: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = explicit_path {
        return Ok(path.to_path_buf());
    }

    if let Some(path) = env::var_os(WORKSPACE_ENV_VAR) {
        return Ok(PathBuf::from(path));
    }

    default_workspace_path().ok_or_else(|| {
        config_error(
            "Could not resolve workspace path because HOME is not set. Set --workspace, PG_LOGSTATS_WORKSPACE, or HOME.",
            Some("workspace"),
        )
    })
}

pub fn workspace_config_path(workspace: &Path) -> PathBuf {
    workspace.join(CONFIG_FILE_NAME)
}

pub fn workspace_inspect_report_path(workspace: &Path) -> PathBuf {
    workspace.join(INSPECT_FILE_NAME)
}

pub fn workspace_results_dir(workspace: &Path) -> PathBuf {
    workspace.join("results")
}

pub fn load_config(explicit_workspace: Option<&Path>) -> Result<ResolvedConfig> {
    if let Some(workspace) = explicit_workspace {
        return load_config_from_workspace(
            workspace,
            ConfigSource::ExplicitWorkspace(workspace.to_path_buf()),
        );
    }

    if let Some(path) = env::var_os(WORKSPACE_ENV_VAR) {
        let workspace = PathBuf::from(path);
        return load_config_from_workspace(
            &workspace,
            ConfigSource::EnvWorkspace(workspace.clone()),
        );
    }

    if let Some(workspace) = default_workspace_path() {
        return load_config_from_workspace(
            &workspace,
            ConfigSource::DefaultWorkspace(workspace.clone()),
        );
    }

    Ok(ResolvedConfig {
        config: AppConfig::default(),
        source: ConfigSource::BuiltInDefaults,
    })
}

fn load_config_from_workspace(workspace: &Path, source: ConfigSource) -> Result<ResolvedConfig> {
    let path = workspace_config_path(workspace);
    if !path.exists() {
        return Ok(ResolvedConfig {
            config: AppConfig::default(),
            source: ConfigSource::BuiltInDefaults,
        });
    }

    if !path.is_file() {
        return Err(config_error(
            &format!("Config path is not a file: {}", path.display()),
            Some("config"),
        ));
    }

    let content = std::fs::read_to_string(&path)?;
    let config: AppConfig =
        toml::from_str(&content).map_err(|err| PgLogstatsError::Configuration {
            message: format!("Failed to parse config {}: {}", path.display(), err),
            field: Some("config".to_string()),
        })?;

    Ok(ResolvedConfig { config, source })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::{Mutex, OnceLock};
    use tempfile::TempDir;

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn write_config(dir: &TempDir, relative_path: &str, content: &str) -> PathBuf {
        let path = dir.path().join(relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn explicit_workspace_has_highest_precedence() {
        let _guard = env_lock().lock().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let explicit_workspace = temp_dir.path().join("explicit-workspace");
        let env_workspace = temp_dir.path().join("env-workspace");
        write_config(
            &temp_dir,
            "explicit-workspace/config.toml",
            "[database]\ndsn='postgres://explicit'\n",
        );
        write_config(
            &temp_dir,
            "env-workspace/config.toml",
            "[database]\ndsn='postgres://env'\n",
        );
        let default = write_config(
            &temp_dir,
            ".local/share/pg-logstats/config.toml",
            "[database]\ndsn='postgres://default'\n",
        );

        let old_home = env::var_os("HOME");
        let old_workspace = env::var_os(WORKSPACE_ENV_VAR);
        env::set_var("HOME", temp_dir.path());
        env::set_var(WORKSPACE_ENV_VAR, &env_workspace);

        let resolved = load_config(Some(&explicit_workspace)).unwrap();

        assert_eq!(
            resolved.source,
            ConfigSource::ExplicitWorkspace(explicit_workspace)
        );
        assert_eq!(
            resolved.config.database.dsn.as_deref(),
            Some("postgres://explicit")
        );

        restore_env(old_home, old_workspace);
        assert!(default.exists());
    }

    #[test]
    fn env_workspace_overrides_default_workspace() {
        let _guard = env_lock().lock().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let env_workspace = temp_dir.path().join("env-workspace");
        write_config(
            &temp_dir,
            "env-workspace/config.toml",
            "[database]\ndsn='postgres://env'\n",
        );
        write_config(
            &temp_dir,
            ".local/share/pg-logstats/config.toml",
            "[database]\ndsn='postgres://default'\n",
        );

        let old_home = env::var_os("HOME");
        let old_workspace = env::var_os(WORKSPACE_ENV_VAR);
        env::set_var("HOME", temp_dir.path());
        env::set_var(WORKSPACE_ENV_VAR, &env_workspace);

        let resolved = load_config(None).unwrap();

        assert_eq!(resolved.source, ConfigSource::EnvWorkspace(env_workspace));
        assert_eq!(
            resolved.config.database.dsn.as_deref(),
            Some("postgres://env")
        );

        restore_env(old_home, old_workspace);
    }

    #[test]
    fn default_workspace_is_used_when_present() {
        let _guard = env_lock().lock().unwrap();
        let temp_dir = TempDir::new().unwrap();
        write_config(
            &temp_dir,
            ".local/share/pg-logstats/config.toml",
            "[database]\ndsn='postgres://default'\n",
        );

        let old_home = env::var_os("HOME");
        let old_workspace = env::var_os(WORKSPACE_ENV_VAR);
        env::set_var("HOME", temp_dir.path());
        env::remove_var(WORKSPACE_ENV_VAR);

        let resolved = load_config(None).unwrap();

        assert_eq!(
            resolved.source,
            ConfigSource::DefaultWorkspace(temp_dir.path().join(".local/share/pg-logstats"))
        );
        assert_eq!(
            resolved.config.database.dsn.as_deref(),
            Some("postgres://default")
        );

        restore_env(old_home, old_workspace);
    }

    #[test]
    fn built_in_defaults_are_used_when_no_config_exists() {
        let _guard = env_lock().lock().unwrap();
        let temp_dir = TempDir::new().unwrap();

        let old_home = env::var_os("HOME");
        let old_workspace = env::var_os(WORKSPACE_ENV_VAR);
        env::set_var("HOME", temp_dir.path());
        env::remove_var(WORKSPACE_ENV_VAR);

        let resolved = load_config(None).unwrap();

        assert_eq!(resolved.source, ConfigSource::BuiltInDefaults);
        assert_eq!(resolved.config, AppConfig::default());

        restore_env(old_home, old_workspace);
    }

    #[test]
    fn unknown_keys_fail_to_parse() {
        let _guard = env_lock().lock().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().join("workspace");
        write_config(
            &temp_dir,
            "workspace/config.toml",
            "unknown_top = true\n[database]\ndsn='postgres://db'\nextra = 1\n[running_queries.thresholds]\nlong_running_query_ms = 3000\nextra = 9\n",
        );

        let err = load_config(Some(&workspace)).unwrap_err();

        assert!(matches!(err, PgLogstatsError::Configuration { .. }));
        assert!(err.to_string().contains("unknown field"));
    }

    #[test]
    fn guidance_max_risk_uses_typed_enum() {
        let _guard = env_lock().lock().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().join("workspace");
        write_config(
            &temp_dir,
            "workspace/config.toml",
            "[guidance]\nmax_risk='safe'\n",
        );

        let resolved = load_config(Some(&workspace)).unwrap();

        assert_eq!(resolved.config.guidance.max_risk, RiskLabel::Safe);
    }

    fn restore_env(
        old_home: Option<std::ffi::OsString>,
        old_workspace: Option<std::ffi::OsString>,
    ) {
        match old_home {
            Some(value) => env::set_var("HOME", value),
            None => env::remove_var("HOME"),
        }

        match old_workspace {
            Some(value) => env::set_var(WORKSPACE_ENV_VAR, value),
            None => env::remove_var(WORKSPACE_ENV_VAR),
        }
    }
}
