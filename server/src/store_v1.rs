use crate::{definitions::ClientObject, definitions::Store, errors::SynxServerError};
use async_trait::async_trait;
use mongodb::{
    bson::{doc, to_document},
    options::ClientOptions,
    Client,
};

pub struct StoreV1 {
    db_client: Client,
    db_name: String,
}

impl StoreV1 {
    /// Creates a new `StoreV1` instance connected to the specified database URL.
    pub async fn new(db_url: &str, db_name: &str) -> Result<Self, SynxServerError> {
        let db_client = Self::create_connections(db_url).await?;
        Ok(Self {
            db_client,
            db_name: db_name.to_string(),
        })
    }

    /// Creates a MongoDB client connection using the provided database URL.
    async fn create_connections(db_url: &str) -> Result<Client, SynxServerError> {
        let client_options = ClientOptions::parse(db_url)
            .await
            .map_err(|err| SynxServerError::DatabaseConnectionError(err.to_string()))?;

        let client = Client::with_options(client_options)
            .map_err(|err| SynxServerError::DbOptionsConfigurationError(err.to_string()))?;

        Ok(client)
    }
}

#[async_trait]
impl Store for StoreV1 {
    async fn get_client_object(&self, id: &str) -> Result<Option<ClientObject>, SynxServerError> {
        let db = self.db_client.database(&self.db_name);
        let collection = db.collection::<ClientObject>("clients");

        let filter = doc! { "id": id };
        let document = collection
            .find_one(filter, None)
            .await
            .map_err(|_| SynxServerError::ClientDataAccessError(id.to_string()))?;

        Ok(document)
    }

    async fn save_client_object(
        &self,
        client_object: ClientObject,
    ) -> Result<bool, SynxServerError> {
        let db = self.db_client.database(&self.db_name);
        let collection = db.collection::<ClientObject>("clients");

        collection
            .insert_one(client_object, None)
            .await
            .map_err(|_| SynxServerError::MongoDbClientCreationError)?;

        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    // use super::*;

    // #[tokio::test]
    // async fn test_db_connection() {
    //     let test_db_url = "mongodb://localhost:27017/test_database";
    //     let result = StoreV1::new(test_db_url).await;
    //     let result = your_collection.find_one(filter, find_options).await?;

    //     assert!(result.is_ok(), "Failed to establish a database connection.");
    // }
}
