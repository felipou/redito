use redito::settings::Commands;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = redito::settings::load_config()?;

    if config.print_config {
        println!("{config:?}")
    }

    match &config.command {
        Commands::None => {}
        Commands::StreamTail(_) => {
            redito::commands::stream_tail::run(config).await?;
        }
        Commands::StreamCopy(_) => {
            redito::commands::stream_copy::run(config).await?;
        }
    }

    Ok(())
}
