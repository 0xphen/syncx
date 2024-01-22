use core::panic;
use futures_util::stream::StreamExt;
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use server::core::{config::Config, errors::SynxServerError, utils::*, worker::Worker};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config =
        Config::load_config().unwrap_or_else(|e| panic!("Failed to load configuration: {}", e));

    let redis_client = connect_redis(&config.redis_url)?;

    let worker_handler = Worker::new(redis_client);
    worker_handler.run_workers().await;

    Ok(())
}
