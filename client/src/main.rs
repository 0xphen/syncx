mod app_config;
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

// { metadata: MetadataMap { headers: {"content-type": "application/grpc", "date": "Thu, 18 Jan 2024 22:47:51 GMT", "grpc-status": "0"} }, message: CreateClientResponse { id: "de93057b-277d-421a-a872-66cccad7788e", jwt_token: "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzUxMiJ9.eyJzdWIiOiJkZTkzMDU3Yi0yNzdkLTQyMWEtYTg3Mi02NmNjY2FkNzc4OGUiLCJleHAiOjE3MDU2MTgxMjYsImlzcyI6IlN5bmN4U2VydmVyIn0.CAhUyRSS454ltAxVRCLvX7ETdKedW7c7pYfUNdGDsSH5xVqRmdPoHiSgMSW22pnFeojQTSXjdmRuc31CPHP0Ag" }, extensions: Extensions }
