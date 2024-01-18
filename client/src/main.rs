mod errors;
mod service;

use proto::syncx::syncx_client::SyncxClient;
use proto::syncx::CreateClientRequest;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = SyncxClient::connect("http://[::1]:10000").await?;
    let res = client
        .register_client(CreateClientRequest {
            password: "Kifen".to_string(),
        })
        .await?;

    println!("res: {:?}", res);
    Ok(())
}
