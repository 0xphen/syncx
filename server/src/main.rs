mod core;

use common::syncx::syncx_server::SyncxServer;
use core::{config::Config, queue::Queue, server::Server as InnerServer, store_v1::StoreV1};
use std::sync::Arc;
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration, panic on failure
    let config =
        Config::load_config().unwrap_or_else(|e| panic!("Failed to load configuration: {}", e));

    let db_client = StoreV1::connect_db(&config.database_url).await?;
    let redis_client = Arc::new(StoreV1::connect_redis(&config.redis_url)?);

    // Initialize StoreV1, panic on failure
    let store_v1 = StoreV1::new(db_client, redis_client.clone(), &config.db_name)
        .await
        .unwrap_or_else(|e| panic!("Failed to initialize StoreV1: {}", e));

    let job_queue = Arc::new(Queue::new(redis_client.clone()));
    // tokio::spawn(async move {
    //     job_queue.clone().run_workers().await;
    // });
    println!("Before...");
    job_queue.clone().run_workers().await;
    println!("AFTER...");
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
