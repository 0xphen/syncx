use super::errors::SynxServerError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ClientObject {
    pub id: String,
    pub password: String,
}

#[async_trait]
pub trait Store {
    async fn get_client_object(&self, id: &str) -> Result<Option<ClientObject>, SynxServerError>;

    async fn save_client_object(
        &self,
        client_object: ClientObject,
    ) -> Result<bool, SynxServerError>;
}
