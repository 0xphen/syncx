mod cli;
mod context;
mod errors;
mod service;

use context::*;
use proto::syncx::syncx_client::SyncxClient;
use proto::syncx::CreateClientRequest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app_config_path = AppConfig::get_config_path()
        .unwrap_or_else(|e| panic!("Failed to get client config path {}", e));

    let app_config = AppConfig::read(&app_config_path)
        .unwrap_or_else(|e| panic!("Failed to load app config: {}", e));

    let mut context = Context::new(app_config, app_config_path);
    let mut syncx_client = SyncxClient::connect("http://[::1]:10000").await?;

    cli::run(&mut syncx_client, &mut context).await;
    Ok(())
}
