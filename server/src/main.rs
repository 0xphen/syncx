mod core;

use tonic::transport::Server;

use core::{config::Config, server::Server as InnerServer, store_v1::StoreV1};
use proto::syncx::syncx_server::SyncxServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration, panic on failure
    let config =
        Config::load_config().unwrap_or_else(|e| panic!("Failed to load configuration: {}", e));

    // Initialize StoreV1, panic on failure
    let store_v1 = StoreV1::new(&config.database_url, &config.db_name)
        .await
        .unwrap_or_else(|e| panic!("Failed to initialize StoreV1: {}", e));

    // Set up the server
    let server = InnerServer::new(store_v1, config).await;
    let addr = "[::1]:10000"
        .parse::<std::net::SocketAddr>()
        .expect("Failed to parse server address");

    let synx_server = SyncxServer::new(server);

    if let Err(e) = Server::builder().add_service(synx_server).serve(addr).await {
        eprintln!("Server failed to start: {}", e);
    } else {
        println!("Server is running on {}", addr);
    }

    Ok(())
}
