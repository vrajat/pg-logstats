use crate::{config_error, triage::RiskLabel, PgLogstatsError, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::{Path, PathBuf};

const CONFIG_ENV_VAR: &str = "PG_LOGSTATS_CONFIG";
const DEFAULT_CONFIG_RELATIVE_PATH: &str = ".config/pg-logstats/config.toml";

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub running_queries: RunningQueriesConfig,
    pub suggest_sql: SuggestSqlConfig,
    pub agent_install: AgentInstallConfig,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DatabaseConfig {
    pub dsn: Option<String>,
    pub connect_timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RunningQueriesConfig {
    pub thresholds: RunningQueriesThresholds,
}

impl Default for RunningQueriesConfig {
    fn default() -> Self {
        Self {
            thresholds: RunningQueriesThresholds::default(),
        }
    }
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
pub struct SuggestSqlConfig {
    pub max_risk: RiskLabel,
    pub show_omitted: bool,
    pub disabled_rules: Vec<String>,
}

impl Default for SuggestSqlConfig {
    fn default() -> Self {
        Self {
            max_risk: RiskLabel::Bounded,
            show_omitted: true,
            disabled_rules: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AgentInstallConfig {
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
    ExplicitPath(PathBuf),
    EnvPath(PathBuf),
    DefaultPath(PathBuf),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedConfig {
    pub config: AppConfig,
    pub source: ConfigSource,
}

pub fn default_config_path() -> Option<PathBuf> {
    env::var_os("HOME").map(|home| PathBuf::from(home).join(DEFAULT_CONFIG_RELATIVE_PATH))
}

pub fn config_env_var_name() -> &'static str {
    CONFIG_ENV_VAR
}

pub fn load_config(explicit_path: Option<&Path>) -> Result<ResolvedConfig> {
    if let Some(path) = explicit_path {
        return load_config_from_path(path, ConfigSource::ExplicitPath(path.to_path_buf()));
    }

    if let Some(path) = env::var_os(CONFIG_ENV_VAR) {
        let path = PathBuf::from(path);
        return load_config_from_path(&path, ConfigSource::EnvPath(path.clone()));
    }

    if let Some(path) = default_config_path() {
        if path.exists() {
            return load_config_from_path(&path, ConfigSource::DefaultPath(path.clone()));
        }
    }

    Ok(ResolvedConfig {
        config: AppConfig::default(),
        source: ConfigSource::BuiltInDefaults,
    })
}

fn load_config_from_path(path: &Path, source: ConfigSource) -> Result<ResolvedConfig> {
    if !path.exists() {
        return Err(config_error(
            &format!("Config file does not exist: {}", path.display()),
            Some("config"),
        ));
    }

    if !path.is_file() {
        return Err(config_error(
            &format!("Config path is not a file: {}", path.display()),
            Some("config"),
        ));
    }

    let content = std::fs::read_to_string(path)?;
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
    fn explicit_path_has_highest_precedence() {
        let _guard = env_lock().lock().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let explicit = write_config(
            &temp_dir,
            "explicit.toml",
            "[database]\ndsn='postgres://explicit'\n",
        );
        let env_path = write_config(&temp_dir, "env.toml", "[database]\ndsn='postgres://env'\n");
        let default = write_config(
            &temp_dir,
            ".config/pg-logstats/config.toml",
            "[database]\ndsn='postgres://default'\n",
        );

        let old_home = env::var_os("HOME");
        let old_env_config = env::var_os(CONFIG_ENV_VAR);
        env::set_var("HOME", temp_dir.path());
        env::set_var(CONFIG_ENV_VAR, &env_path);

        let resolved = load_config(Some(&explicit)).unwrap();

        assert_eq!(resolved.source, ConfigSource::ExplicitPath(explicit));
        assert_eq!(
            resolved.config.database.dsn.as_deref(),
            Some("postgres://explicit")
        );

        restore_env(old_home, old_env_config);
        assert!(default.exists());
    }

    #[test]
    fn env_path_overrides_default_path() {
        let _guard = env_lock().lock().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let env_path = write_config(&temp_dir, "env.toml", "[database]\ndsn='postgres://env'\n");
        write_config(
            &temp_dir,
            ".config/pg-logstats/config.toml",
            "[database]\ndsn='postgres://default'\n",
        );

        let old_home = env::var_os("HOME");
        let old_env_config = env::var_os(CONFIG_ENV_VAR);
        env::set_var("HOME", temp_dir.path());
        env::set_var(CONFIG_ENV_VAR, &env_path);

        let resolved = load_config(None).unwrap();

        assert_eq!(resolved.source, ConfigSource::EnvPath(env_path));
        assert_eq!(
            resolved.config.database.dsn.as_deref(),
            Some("postgres://env")
        );

        restore_env(old_home, old_env_config);
    }

    #[test]
    fn default_path_is_used_when_present() {
        let _guard = env_lock().lock().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let default = write_config(
            &temp_dir,
            ".config/pg-logstats/config.toml",
            "[database]\ndsn='postgres://default'\n",
        );

        let old_home = env::var_os("HOME");
        let old_env_config = env::var_os(CONFIG_ENV_VAR);
        env::set_var("HOME", temp_dir.path());
        env::remove_var(CONFIG_ENV_VAR);

        let resolved = load_config(None).unwrap();

        assert_eq!(resolved.source, ConfigSource::DefaultPath(default));
        assert_eq!(
            resolved.config.database.dsn.as_deref(),
            Some("postgres://default")
        );

        restore_env(old_home, old_env_config);
    }

    #[test]
    fn built_in_defaults_are_used_when_no_config_exists() {
        let _guard = env_lock().lock().unwrap();
        let temp_dir = TempDir::new().unwrap();

        let old_home = env::var_os("HOME");
        let old_env_config = env::var_os(CONFIG_ENV_VAR);
        env::set_var("HOME", temp_dir.path());
        env::remove_var(CONFIG_ENV_VAR);

        let resolved = load_config(None).unwrap();

        assert_eq!(resolved.source, ConfigSource::BuiltInDefaults);
        assert_eq!(resolved.config, AppConfig::default());

        restore_env(old_home, old_env_config);
    }

    #[test]
    fn unknown_keys_fail_to_parse() {
        let _guard = env_lock().lock().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let config_path = write_config(
            &temp_dir,
            "custom.toml",
            "unknown_top = true\n[database]\ndsn='postgres://db'\nextra = 1\n[running_queries.thresholds]\nlong_running_query_ms = 3000\nextra = 9\n",
        );

        let err = load_config(Some(&config_path)).unwrap_err();

        assert!(matches!(err, PgLogstatsError::Configuration { .. }));
        assert!(err.to_string().contains("unknown field"));
    }

    #[test]
    fn suggest_sql_max_risk_uses_typed_enum() {
        let _guard = env_lock().lock().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let config_path =
            write_config(&temp_dir, "custom.toml", "[suggest_sql]\nmax_risk='safe'\n");

        let resolved = load_config(Some(&config_path)).unwrap();

        assert_eq!(resolved.config.suggest_sql.max_risk, RiskLabel::Safe);
    }

    fn restore_env(
        old_home: Option<std::ffi::OsString>,
        old_env_config: Option<std::ffi::OsString>,
    ) {
        match old_home {
            Some(value) => env::set_var("HOME", value),
            None => env::remove_var("HOME"),
        }

        match old_env_config {
            Some(value) => env::set_var(CONFIG_ENV_VAR, value),
            None => env::remove_var(CONFIG_ENV_VAR),
        }
    }
}
