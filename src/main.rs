use reto::settings::Commands;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = reto::settings::load_config()?;

    if config.print_config {
        println!("{config:?}")
    }

    match &config.command {
        Commands::None => {}
        Commands::StreamTail(_) => {
            reto::commands::stream_tail::run(config).await?;
        }
        Commands::StreamCopy(_) => {
            reto::commands::stream_copy::run(config).await?;
        }
    }

    Ok(())
}
