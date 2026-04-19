mod api;
mod config;
mod error;
mod logind;

use clap::Parser;
use config::Config;
use logind::client::ZbusLogindClient;
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "russessin", about = "REST API for systemd-logind operations")]
struct Cli {
    /// Path to configuration file
    #[arg(short, long, default_value = "/etc/russessin/russessin.toml")]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let config = if cli.config.exists() {
        Config::from_file(&cli.config)?
    } else {
        tracing::warn!(
            "Config file {} not found, using defaults",
            cli.config.display()
        );
        Config::default()
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new(&config.logging.level)),
        )
        .init();

    tracing::info!("Starting russessin on {}", config.bind_address());

    let client = ZbusLogindClient::new().await?;
    let client = Arc::new(client) as Arc<dyn logind::LogindClient>;

    let app = api::router(client);

    let listener = tokio::net::TcpListener::bind(&config.bind_address()).await?;
    tracing::info!("Listening on {}", config.bind_address());
    axum::serve(listener, app).await?;

    Ok(())
}
