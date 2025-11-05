use std::{collections::HashMap, io::Write};

use crate::settings::{AppConfig, Commands, StreamTailArgs};
use redis::{
    FromRedisValue,
    streams::{StreamReadOptions, StreamReadReply},
};
use serde_json::json;

fn print_stream_read_reply(
    reply: &StreamReadReply,
    as_json: bool,
    raw_key: Option<&str>,
) -> anyhow::Result<()> {
    for stream in &reply.keys {
        for entry in &stream.ids {
            if let Some(raw_key) = raw_key {
                let value = entry
                    .map
                    .get(raw_key)
                    .map(String::from_redis_value)
                    .transpose()?
                    .unwrap_or(String::from("null"));
                println!("{}", value);
                continue;
            }

            let id = &entry.id;
            let kvs: Result<HashMap<&str, String>, redis::RedisError> = entry
                .map
                .iter()
                .map(|(key, val)| String::from_redis_value(val).map(|v| (key.as_str(), v)))
                .collect();

            if as_json {
                let value = json!({
                    "id": id,
                    "map": kvs?,
                });
                println!("{}", value);
            } else {
                println!("ID: {}", id);
                for (key, val) in kvs?.iter() {
                    println!("  {} => {}", key, val);
                }
            }
        }
    }
    std::io::stdout().flush()?;

    Ok(())
}

async fn xread(
    conn: &mut redis::aio::MultiplexedConnection,
    last_id: &str,
    config: &StreamTailArgs,
) -> redis::RedisResult<redis::streams::StreamReadReply> {
    if let Some(group) = &config.group {
        let consumer = config.consumer.as_deref().unwrap_or("default");
        let options = StreamReadOptions::default()
            .block(config.block_ms)
            .count(config.count)
            .group(group, consumer);
        let last_id = ">";
        redis::AsyncCommands::xread_options(conn, &[&config.stream], &[&last_id], &options).await
    } else {
        let options = StreamReadOptions::default()
            .block(config.block_ms)
            .count(config.count);
        redis::AsyncCommands::xread_options(conn, &[&config.stream], &[&last_id], &options).await
    }
}

pub async fn run(config: AppConfig) -> anyhow::Result<()> {
    let Commands::StreamTail(command_config) = config.command else {
        panic!("Invalid command config received!");
    };

    if config.redis.sentinel && config.redis.sentinel_master.is_none() {
        eprintln!("--sentinel requires --sentinel-master to be set");
        std::process::exit(1);
    }

    let mut conn = crate::connect::connect(&config.redis).await?;

    let mut last_id = command_config.start_id.clone();

    loop {
        let reply = xread(&mut conn, &last_id, &command_config).await?;

        print_stream_read_reply(
            &reply,
            !command_config.plaintext,
            command_config.raw_key.as_deref(),
        )?;

        if reply.keys.is_empty() && !command_config.retry_when_empty {
            break;
        }

        for stream in reply.keys {
            for entry in stream.ids {
                last_id = entry.id;
            }
        }
    }
    Ok(())
}
