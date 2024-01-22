use server::core::{config::Config, queue::Queue};
use tokio;

#[tokio::main]
async fn main() {
    let config =
        Config::load_config().unwrap_or_else(|e| panic!("Failed to load configuration: {}", e));
}
