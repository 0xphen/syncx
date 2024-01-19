use super::errors::SynxServerError;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub type Result<T> = std::result::Result<T, SynxServerError>;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ClientObject {
    pub id: String,
    pub password: String,
}

#[async_trait]
pub trait Store {
    async fn get_client_object(&self, id: &str) -> Result<Option<ClientObject>>;

    async fn save_client_object(&self, client_object: ClientObject) -> Result<bool>;
}
