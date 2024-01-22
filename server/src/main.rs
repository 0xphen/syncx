mod core;

use common::{common::logger_init, syncx::syncx_server::SyncxServer};
use core::{config::Config, server::Server as InnerServer, store_v1::StoreV1, utils::*};
use log::{error, info};
use std::env;
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    logger_init(Some(&std::env::var("LOG_CONFIG").unwrap()));
    info!("Initializing sync server...");

    // Load configuration, panic on failure
    let config =
        Config::load_config().unwrap_or_else(|e| panic!("Failed to load configuration: {}", e));

    let db_client = connect_db(&config.database_url).await?;
    let redis_client = connect_redis(&config.redis_url)?;

    // Initialize StoreV1, panic on failure
    let store_v1 = StoreV1::new(db_client, redis_client, &config.db_name)
        .await
        .unwrap_or_else(|e| panic!("Failed to initialize StoreV1: {}", e));

    // Set up the server
    let server = InnerServer::new(store_v1, config).await;

    let addr = std::env::var("SERVER_ADDR")
        .unwrap()
        .parse::<std::net::SocketAddr>()
        .expect("Failed to parse server address");

    let synx_server = SyncxServer::new(server);

    info!("Server is running on address {}", addr);
    if let Err(e) = Server::builder().add_service(synx_server).serve(addr).await {
        error!("Server failed to start due to {}", e)
    }

    Ok(())
}
