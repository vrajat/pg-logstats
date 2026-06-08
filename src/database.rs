use crate::config::AppConfig;
use postgres::{Client, NoTls};
use std::env;
use std::str::FromStr;
use std::time::Duration;

const DATABASE_URL_ENV_VAR: &str = "PG_LOGSTATS_DATABASE_URL";

pub fn database_url_env_var_name() -> &'static str {
    DATABASE_URL_ENV_VAR
}

pub fn resolve_database_dsn(cli_dsn: Option<&str>, config: &AppConfig) -> Option<String> {
    cli_dsn
        .map(str::to_string)
        .or_else(|| env::var(DATABASE_URL_ENV_VAR).ok())
        .or_else(|| config.database.dsn.clone())
}

pub fn connect_postgres_client(
    dsn: &str,
    connect_timeout_ms: Option<u64>,
) -> Result<Client, String> {
    let mut config = postgres::Config::from_str(dsn)
        .map_err(|err| format!("database_connection_invalid: {err}"))?;
    if let Some(connect_timeout_ms) = connect_timeout_ms {
        config.connect_timeout(Duration::from_millis(connect_timeout_ms));
    }
    config
        .connect(NoTls)
        .map_err(|err| format!("database_connection_failed: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DatabaseConfig;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn cli_dsn_has_highest_precedence() {
        let _guard = env_lock().lock().unwrap();
        let config = AppConfig {
            database: DatabaseConfig {
                dsn: Some("postgres://config".to_string()),
                connect_timeout_ms: None,
            },
            ..AppConfig::default()
        };
        env::set_var(DATABASE_URL_ENV_VAR, "postgres://env");

        assert_eq!(
            resolve_database_dsn(Some("postgres://cli"), &config).as_deref(),
            Some("postgres://cli")
        );

        env::remove_var(DATABASE_URL_ENV_VAR);
    }

    #[test]
    fn env_dsn_overrides_config() {
        let _guard = env_lock().lock().unwrap();
        let config = AppConfig {
            database: DatabaseConfig {
                dsn: Some("postgres://config".to_string()),
                connect_timeout_ms: None,
            },
            ..AppConfig::default()
        };
        env::set_var(DATABASE_URL_ENV_VAR, "postgres://env");

        assert_eq!(
            resolve_database_dsn(None, &config).as_deref(),
            Some("postgres://env")
        );

        env::remove_var(DATABASE_URL_ENV_VAR);
    }
}
