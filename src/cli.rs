use std::{ffi::OsString, path::PathBuf};

use clap::{Args, Parser, Subcommand};

use serde::Serialize;

#[derive(Debug, Clone, Serialize, Args)]
pub(crate) struct RedisConfig {
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,

    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,

    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,

    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,

    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub db: Option<u8>,

    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<bool>,

    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sentinel: Option<bool>,

    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sentinel_master: Option<String>,
}

#[derive(Debug, Clone, Serialize, Args)]
pub(crate) struct TargetRedisConfig {
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "host"))]
    pub target_host: Option<String>,

    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "port"))]
    pub target_port: Option<u16>,

    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "username"))]
    pub target_username: Option<String>,

    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "password"))]
    pub target_password: Option<String>,

    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "db"))]
    pub target_db: Option<u8>,

    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "tls"))]
    pub target_tls: Option<bool>,

    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "root_cert"))]
    pub target_root_cert: Option<String>,

    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "client_cert"))]
    pub target_client_cert: Option<String>,

    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "client_key"))]
    pub target_client_key: Option<String>,

    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "sentinel"))]
    pub target_sentinel: Option<bool>,

    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "sentinel_master"))]
    pub target_sentinel_master: Option<String>,
}

#[derive(Debug, Clone, Serialize, Subcommand)]
#[serde(rename_all = "snake_case")]
#[serde(tag = "command_type")]
pub enum Commands {
    StreamTail(StreamTailArgs),

    StreamCopy(StreamCopyArgs),

    #[clap(hide = true)]
    None,
}

#[derive(Debug, Clone, Serialize, Args)]
pub struct StreamTailArgs {
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) stream: Option<String>,

    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) plaintext: Option<bool>,

    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) raw_key: Option<String>,

    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) group: Option<String>,

    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) consumer: Option<String>,

    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) block_ms: Option<usize>,

    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) count: Option<usize>,

    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) start_id: Option<String>,

    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) retry_when_empty: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Args)]
pub struct StreamCopyArgs {
    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) stream: Option<String>,

    #[command(flatten)]
    pub(crate) target: TargetRedisConfig,

    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) group: Option<String>,

    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) consumer: Option<String>,

    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) block_ms: Option<usize>,

    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) count: Option<usize>,

    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) start_id: Option<String>,

    #[arg(long)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) retry_when_empty: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Parser)]
#[command(name = "reto", author, version, about)]
pub struct AppCliConfig {
    #[arg(long)]
    pub config: Option<PathBuf>,

    #[command(flatten)]
    pub(crate) redis: RedisConfig,

    #[command(subcommand)]
    pub command: Commands,
}

pub fn parse_args(args: impl Iterator<Item = OsString>) -> anyhow::Result<AppCliConfig> {
    Ok(AppCliConfig::try_parse_from(args)?)
}

pub fn parse() -> anyhow::Result<AppCliConfig> {
    parse_args(std::env::args_os())
}
