use std::path::PathBuf;

use figment::{
    Figment,
    providers::{Env, Format, Serialized, Toml},
};
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RedisConfig {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub db: u8,
    pub tls: bool,
    pub sentinel: bool,
    pub sentinel_master: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "command_type")]
pub enum Commands {
    StreamTail(StreamTailArgs),

    StreamCopy(StreamCopyArgs),

    None,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StreamTailArgs {
    pub(crate) stream: String,
    pub(crate) plaintext: bool,
    pub(crate) raw_key: Option<String>,
    pub(crate) group: Option<String>,
    pub(crate) consumer: Option<String>,
    pub(crate) block_ms: usize,
    pub(crate) count: usize,
    pub(crate) start_id: String,
    pub(crate) retry_when_empty: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StreamCopyArgs {
    pub(crate) stream: String,
    pub(crate) target: RedisConfig,
    pub(crate) group: Option<String>,
    pub(crate) consumer: Option<String>,
    pub(crate) block_ms: usize,
    pub(crate) count: usize,
    pub(crate) start_id: String,
    pub(crate) retry_when_empty: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub(crate) redis: RedisConfig,
    pub command: Commands,
}

const CONFIG_FILE_BASE_NAME: &str = "reto";
const ENV_VAR_PREFIX: &str = "RETO_";

fn default_json() -> serde_json::Value {
    json!({
        "redis": {
            "host": "localhost",
            "port": 6379,
            "db": 0,
            "tls": false,
            "sentinel": false,
        },
        "command": {
            // stream_tail
            "plaintext": false,

            // stream_copy
            "target": {
                "port": 6379,
                "db": 0,
                "tls": false,
                "sentinel": false,
            },

            // both stream commands above
            "block_ms": 5000,
            "count": 1000,
            "start_id": "$",
            "retry_when_empty": false,
        }
    })
}

pub fn load_config() -> anyhow::Result<AppConfig> {
    let cli_config = crate::cli::parse()?;

    let file_config: AppConfig = Figment::from(Serialized::defaults(json!({
        "redis": {
            "host": "localhost",
            "port": 6379,
            "db": 0,
            "tls": false,
            "sentinel": false,
        },
        "command": {
            "command_type": "none",
        },
    })))
    .merge(Toml::file(format!("/etc/{CONFIG_FILE_BASE_NAME}.toml")))
    .merge(Toml::file(format!(".{CONFIG_FILE_BASE_NAME}.toml")))
    .merge(Toml::file(cli_config.config.clone().unwrap_or(
        PathBuf::from(format!("{CONFIG_FILE_BASE_NAME}.toml")),
    )))
    .merge(Toml::file("local_config.toml"))
    .merge(Env::prefixed(ENV_VAR_PREFIX).lowercase(true).split("__"))
    .merge(Serialized::defaults(cli_config.clone()))
    .join(Serialized::defaults(default_json()))
    .extract()?;

    Ok(file_config)
}
