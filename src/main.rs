use anyhow::Result;
use clap::{Arg, Command};
use podcast_tui::{app::App, config::Config};

#[tokio::main]
async fn main() -> Result<()> {
    let matches = Command::new("podcast-tui")
        .version("1.0.0-mvp")
        .about("A cross-platform terminal user interface for podcast management")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Sets a custom config file"),
        )
        .get_matches();

    let config_path = matches.get_one::<String>("config");
    let config = Config::load_or_default(config_path)?;

    let mut app = App::new(config).await?;
    app.run().await?;

    Ok(())
}
