use anyhow::Context;
use figment::{
    Figment,
    providers::{Env, Format, Serialized, Toml},
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use crate::cli::setup_from_schema;

#[derive(Debug, Clone, Deserialize, JsonSchema)]
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

#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "command_type")]
pub enum Commands {
    StreamTail(StreamTailArgs),

    StreamCopy(StreamCopyArgs),

    None,
}

#[derive(Debug, Clone, Deserialize, JsonSchema)]
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

#[derive(Debug, Clone, Deserialize, JsonSchema)]
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

#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct AppConfig {
    pub(crate) redis: RedisConfig,
    pub command: Commands,

    pub print_config: bool,
}

const CONFIG_FILE_BASE_NAME: &str = "reto";
const ENV_VAR_PREFIX: &str = "RETO_";

fn default_json() -> serde_json::Value {
    json!({
        "print_config": false,
        "redis": {
            "host": "localhost",
            "port": 6379,
            "db": 0,
            "tls": false,
            "sentinel": false,
        },
        "command": {
            "command_type": "none",

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
    let figment = Figment::from(Serialized::defaults(default_json()))
        .merge(Toml::file(format!("/etc/{CONFIG_FILE_BASE_NAME}.toml")))
        .merge(Toml::file(format!(".{CONFIG_FILE_BASE_NAME}.toml")))
        .merge(Toml::file(format!("{CONFIG_FILE_BASE_NAME}.toml")))
        .merge(Env::prefixed(ENV_VAR_PREFIX).lowercase(true).split("__"));

    let base_config: serde_json::Value =
        figment.extract().context("Failed to extract base config")?;

    let cli_json = setup_from_schema(base_config)?;

    let final_figment = figment.merge(Serialized::defaults(cli_json));
    let final_config: AppConfig = final_figment
        .extract()
        .context("Failed to extract final config")?;
    Ok(final_config)
}
