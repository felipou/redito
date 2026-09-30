use crate::settings::{AppConfig, Commands, StreamCopyArgs};
use redis::streams::{StreamAddOptions, StreamReadOptions};

type RawStreamReadResponse = Vec<(String, Vec<(String, Vec<(String, Vec<u8>)>)>)>;

async fn xread(
    conn: &mut redis::aio::MultiplexedConnection,
    last_id: &str,
    config: &StreamCopyArgs,
) -> redis::RedisResult<RawStreamReadResponse> {
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

async fn xadd_all(
    conn: &mut redis::aio::MultiplexedConnection,
    data: &RawStreamReadResponse,
) -> redis::RedisResult<()> {
    let options = StreamAddOptions::default();

    let mut pipe = redis::pipe();

    for (stream_name, entries) in data {
        for (entry_id, entry_items) in entries {
            pipe.xadd_options(stream_name, entry_id, entry_items, &options);
        }
    }

    pipe.exec_async(conn).await?;

    Ok(())
}

pub async fn run(config: AppConfig) -> anyhow::Result<()> {
    let Commands::StreamCopy(command_config) = config.command else {
        panic!("Invalid command config received!");
    };

    if config.redis.sentinel && config.redis.sentinel_master.is_none() {
        eprintln!("--sentinel requires --sentinel-master to be set");
        std::process::exit(1);
    }

    // Installed before connecting, so a stop is honoured at any point.
    let mut shutdown = crate::shutdown::Shutdown::install()?;

    let connect = async {
        let source_conn = crate::connect::connect(&config.redis).await?;
        let target_conn = crate::connect::connect(&command_config.target).await?;
        redis::RedisResult::Ok((source_conn, target_conn))
    };
    let (mut source_conn, mut target_conn) = tokio::select! {
        signal = shutdown.recv() => {
            eprintln!("Received {signal} while connecting, exiting");
            return Ok(());
        }
        conns = connect => conns?,
    };

    let mut last_id = command_config.start_id.clone();

    loop {
        // Stop between batches: a signal interrupts the (blocking) XREAD, but never a batch
        // that was already read and not yet written to the target.
        let reply = tokio::select! {
            biased;
            signal = shutdown.recv() => {
                eprintln!("Received {signal}, stopping");
                break;
            }
            reply = xread(&mut source_conn, &last_id, &command_config) => reply?,
        };

        xadd_all(&mut target_conn, &reply).await?;

        if reply.is_empty() && !command_config.retry_when_empty {
            break;
        }

        for (_stream_name, entries) in reply {
            for (entry_id, _entry_items) in entries {
                last_id = entry_id;
            }
        }
    }
    Ok(())
}
