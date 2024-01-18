pub mod client {
    extern crate proto;

    use crate::errors::SynxClientError;
    use proto::syncx::{syncx_client::SyncxClient, CreateClientRequest, CreateClientResponse};

    pub async fn register_client<T>(
        syncx_client: &mut SyncxClient<tonic::transport::Channel>,
        password: &str,
    ) -> Result<String, SynxClientError> {
        let response = syncx_client
            .register_client(CreateClientRequest {
                password: password.to_string(),
            })
            .await
            .map_err(|err| SynxClientError::FailedToRegisterClient(err.to_string()))?;

          Ok(response.into_inner().id)


    }
}

