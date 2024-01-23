mod cli;
mod core;

use common::syncx::syncx_client::SyncxClient;
use core::context::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app_config_path = AppConfig::get_config_path()
        .unwrap_or_else(|e| panic!("Failed to get client config path {}", e));
    let app_config = match AppConfig::read(&app_config_path) {
        Ok(app_config) => app_config,
        Err(_) => {
            println!("Client config not found. Creating new config...");
            println!("Config saved to {:?}", app_config_path);

            let app_config = AppConfig::default();
            let _ = app_config.write(&app_config_path);
            app_config
        }
    };

    let mut context = Context::new(app_config, app_config_path);
    let mut syncx_client = SyncxClient::connect("http://[::1]:10000").await?;

    cli::run(&mut syncx_client, &mut context).await;
    Ok(())
}
