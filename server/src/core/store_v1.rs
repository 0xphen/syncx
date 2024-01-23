use super::{
    definitions::{
        ClientObject, R2D2Pool, RedisPool, Result, Store, CACHE_POOL_TIMEOUT_SECONDS, JOB_QUEUE,
    },
    errors::SynxServerError,
};

use async_trait::async_trait;
use log::error;
use mongodb::{bson::doc, Client};
use r2d2_redis::redis::Commands;
use serde_json;

pub struct StoreV1 {
    db_client: Client,
    db_name: String,
    redis_pool: R2D2Pool,
}

impl RedisPool for StoreV1 {
    fn get_pool(&self) -> &R2D2Pool {
        &self.redis_pool
    }
}

impl StoreV1 {
    /// Creates a new `StoreV1` instance connected to the specified database URL.
    pub async fn new(db_client: Client, redis_pool: R2D2Pool, db_name: &str) -> Result<Self> {
        Ok(Self {
            db_name: db_name.to_string(),
            redis_pool,
            db_client,
        })
    }

    async fn fetch_client_object_from_db(&self, id: &str) -> Result<Option<ClientObject>> {
        let db = self.db_client.database(&self.db_name);
        let collection = db.collection::<ClientObject>("clients");

        let filter = doc! { "id": id };
        let document = collection
            .find_one(filter, None)
            .await
            .map_err(|_| SynxServerError::ClientDataAccessError(id.to_string()))?;

        Ok(document)
    }

    pub fn save_to_cache(&self, key: &str, value: &str) -> Result<()> {
        let mut conn = self.get_redis_connection(CACHE_POOL_TIMEOUT_SECONDS)?;
        conn.set(key, value)
            .map_err(|err| SynxServerError::RedisCMDError(err.to_string()))?;

        Ok(())
    }
}

#[async_trait]
impl Store for StoreV1 {
    fn fetch_from_cache(&self, key: &str) -> Result<Option<String>> {
        let mut conn = self.get_redis_connection(CACHE_POOL_TIMEOUT_SECONDS)?;

        let value = conn.get::<&str, Option<String>>(key).map_err(|err| {
            error!("error retrieving key {}", key);
            SynxServerError::RedisCMDError(err.to_string())
        })?;

        Ok(value)
    }

    async fn get_client_object(&self, id: &str) -> Result<Option<ClientObject>> {
        let value = self.fetch_from_cache(id)?;
        match value {
            Some(value) => {
                let client_object: ClientObject = serde_json::from_str(&value)
                    .map_err(|err| SynxServerError::DeserializationError(err.to_string()))?;

                Ok(Some(client_object))
            }
            None => {
                let client_object = self.fetch_client_object_from_db(id).await?;

                if let Some(ref value) = client_object {
                    let json_string = serde_json::to_string(value)
                        .map_err(|err| SynxServerError::SerializationError(err.to_string()))?;

                    self.save_to_cache(id, &json_string)?;
                }

                Ok(client_object)
            }
        }
    }

    async fn save_client_object(&self, client_object: ClientObject) -> Result<bool> {
        let db = self.db_client.database(&self.db_name);
        let collection = db.collection::<ClientObject>("clients");

        collection
            .insert_one(client_object, None)
            .await
            .map_err(|_| SynxServerError::MongoDbClientCreationError)?;

        Ok(true)
    }

    fn enqueue_job(&self, value: &str) -> Result<()> {
        let mut conn = self.get_redis_connection(CACHE_POOL_TIMEOUT_SECONDS)?;

        let value = conn
            .rpush::<&str, &str, String>(JOB_QUEUE, value)
            .map_err(|err| SynxServerError::DequeueJobError(err.to_string()))?;

        println!("New job queued: {:?}", value);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dotenv::dotenv;
    use r2d2_redis::redis::ConnectionLike;
    use std::env;

    use lazy_static::lazy_static;
    lazy_static! {
        static ref REDIS_URL: String =
            env::var("TEST_REDIS_URL").expect("TEST_REDIS_URL must be set");
        static ref DATABASE_URL: String =
            env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        static ref DB_NAME: String = env::var("DB_NAME").expect("DB_NAME must be set");
    }

    const ID: &str = "uuid";
    const PASSWORD: &str = "uuid";

    async fn flush_cache_db(store: &mut StoreV1) {
        // Clear the DB
        store.db_client.database(&DB_NAME).drop(None).await;

        // Clear the redis cache
        let mut pool_conn = store
            .redis_pool
            .get()
            .expect("Failed to get Redis connection");

        pool_conn.req_command(&cmd("FLUSHDB"));
    }

    async fn setup() -> StoreV1 {
        dotenv::from_filename(".env.test").ok();
        let db_client = connect_db(&DATABASE_URL).await.unwrap();
        let redis_client = connect_redis(&REDIS_URL).unwrap();

        let mut store_v1 = StoreV1::new(db_client, redis_client, &DB_NAME)
            .await
            .unwrap();

        flush_cache_db(&mut store_v1).await;

        store_v1
    }

    #[tokio::test]
    async fn cache_test() {
        let store_v1 = setup().await;

        let client = ClientObject {
            id: ID.to_string(),
            password: PASSWORD.to_string(),
        };

        let (key, value) = ("key", serde_json::to_string(&client).unwrap());
        let _ = store_v1.save_to_cache(key, &value);
        let value = store_v1.fetch_from_cache(key).unwrap();
        let value = serde_json::from_str::<ClientObject>(&value.unwrap()).unwrap();

        assert!(value == client);
    }
}
