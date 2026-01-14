use std::str::FromStr;

use color_eyre::eyre::WrapErr;
use tracing::Level;

pub struct Config {
    pub log_level: Level,
    pub listen_addr: String,
    pub frigate_url: String,
    pub frigate_user: String,
    pub frigate_password: String,
}

fn env(key: &str) -> color_eyre::Result<String> {
    std::env::var(key).wrap_err_with(|| format!("failed to read environment variable {key}"))
}

impl Config {
    pub fn load_from_env() -> color_eyre::Result<Config> {
        let log_level = env("ALAAARM_LOG")
            .ok()
            .and_then(|x| Level::from_str(&x).ok())
            .unwrap_or(Level::INFO);

        let listen_addr = env("ALAAARM_LISTEN").unwrap_or_else(|_| "0.0.0.0:6060".into());
        let frigate_url = env("ALAAARM_FRIGATE_URL")?;
        let frigate_user = env("ALAAARM_FRIGATE_USER")?;
        let frigate_password = env("ALAAARM_FRIGATE_PASSWORD")?;

        Ok(Self {
            log_level,
            listen_addr,
            frigate_url,
            frigate_user,
            frigate_password,
        })
    }
}
