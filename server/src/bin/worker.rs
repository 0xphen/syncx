use common::common::logger_init;
use core::panic;


use server::core::{config::Config, utils::*, worker::Worker};



use std::sync::Arc;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    logger_init(Some(&std::env::var("LOG_CONFIG").unwrap()));

    let config =
        Config::load_config().unwrap_or_else(|e| panic!("Failed to load configuration: {}", e));

    let redis_client = Arc::new(connect_redis(&config.redis_url)?);

    let worker_handler = Arc::new(Worker::new(redis_client));
    worker_handler.run_workers().await;

    Ok(())
}
