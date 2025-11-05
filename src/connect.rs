use crate::settings::RedisConfig;

fn build_redis_url(args: &RedisConfig) -> String {
    let scheme = if args.tls { "rediss" } else { "redis" };
    let auth = match (&args.username, &args.password) {
        (Some(u), Some(p)) => format!("{u}:{p}@"),
        (None, Some(p)) => format!(":{p}@"),
        _ => "".to_string(),
    };

    format!("{scheme}://{auth}{}:{}/{}", args.host, args.port, args.db)
}

pub async fn connect(
    config: &RedisConfig,
) -> redis::RedisResult<redis::aio::MultiplexedConnection> {
    if config.sentinel {
        let mut sentinel = redis::sentinel::SentinelClient::build(
            vec![(config.host.as_str(), config.port)],
            config.sentinel_master.clone().unwrap(),
            None,
            redis::sentinel::SentinelServerType::Master,
        )?;
        sentinel.get_async_connection().await
    } else {
        let client = redis::Client::open(build_redis_url(config))?;
        client.get_multiplexed_async_connection().await
    }
}
