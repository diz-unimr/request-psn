mod api;
mod auth;
mod client;
mod config;
mod error;
mod server;

use crate::config::AppConfig;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // config
    let config = AppConfig::new().expect("Failed to load config");

    // run
    server::serve(config).await?;

    Ok(())
}
