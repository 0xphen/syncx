use super::{definitions::ClientObject, definitions::Store, errors::SynxServerError};

use async_trait::async_trait;
use mongodb::{
    bson::{doc, to_document},
    options::ClientOptions,
    Client,
};
use r2d2_redis::{
    r2d2,
    redis::{cmd, Commands, FromRedisValue, Value},
    RedisConnectionManager,
};
use redis::RedisResult;
use serde_json;
use std::{thread::sleep, time::Duration};

use super::definitions::Result;

pub type R2D2Pool = r2d2::Pool<RedisConnectionManager>;
pub type R2D2Con = r2d2::PooledConnection<RedisConnectionManager>;

const CACHE_POOL_MAX_OPEN: u32 = 16;
const CACHE_POOL_MIN_IDLE: u32 = 8;
const CACHE_POOL_TIMEOUT_SECONDS: u64 = 1;
const CACHE_POOL_EXPIRE_SECONDS: u64 = 60;

pub struct StoreV1 {
    db_client: Client,
    db_name: String,
    redis_pool: R2D2Pool,
}

impl StoreV1 {
    /// Creates a new `StoreV1` instance connected to the specified database URL.
    pub async fn new(db_url: &str, redis_url: &str, db_name: &str) -> Result<Self> {
        let db_client = Self::connect_db(db_url).await?;
        let redis_pool = Self::connect_redis(redis_url)?;

        Ok(Self {
            db_client,
            db_name: db_name.to_string(),
            redis_pool,
        })
    }

    fn connect_redis(url: &str) -> Result<R2D2Pool> {
        let manager = RedisConnectionManager::new(url)
            .map_err(|err| SynxServerError::RedisConnectionError(err.to_string()))?;

        let pool_manager = r2d2::Pool::builder()
            .max_size(CACHE_POOL_MAX_OPEN)
            .max_lifetime(Some(Duration::from_secs(CACHE_POOL_EXPIRE_SECONDS)))
            .min_idle(Some(CACHE_POOL_MIN_IDLE))
            .build(manager)
            .map_err(|err| SynxServerError::RedisConnectionError(err.to_string()))?;

        Ok(pool_manager)
    }

    /// Asynchronously creates a database client connection.
    /// Establishes a connection to the database specified by `db_url`.
    /// Returns a `Client` on success or a `SynxServerError` on failure.
    ///
    /// # Arguments
    /// * `db_url` - A string slice that holds the database connection URL.
    ///
    /// # Returns
    /// A `Result<Client, SynxServerError>`:
    /// - `Ok(Client)`: Database client on successful connection.
    /// - `Err(SynxServerError)`: An error of type `DatabaseConnectionError` if the connection fails,
    ///   or `DbOptionsConfigurationError` if there's an error configuring the database options.
    ///
    /// # Errors
    /// This function will return an error if:
    /// - The database URL is invalid or the server is unreachable (returns `DatabaseConnectionError`).
    /// - There's an error setting up the client options (returns `DbOptionsConfigurationError`).
    async fn connect_db(db_url: &str) -> Result<Client> {
        let client_options = ClientOptions::parse(db_url)
            .await
            .map_err(|err| SynxServerError::DatabaseConnectionError(err.to_string()))?;

        let client = Client::with_options(client_options)
            .map_err(|err| SynxServerError::DbOptionsConfigurationError(err.to_string()))?;

        Ok(client)
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

    fn get_redis_connection(&self) -> Result<R2D2Con> {
        self.redis_pool
            .get_timeout(Duration::from_secs(CACHE_POOL_TIMEOUT_SECONDS))
            .map_err(|e| {
                eprintln!("error connecting to redis: {}", e);
                SynxServerError::RedisPoolError(e.to_string())
            })
    }

    fn fetch_from_cache(&self, key: &str) -> Result<Option<String>> {
        let mut conn = self.get_redis_connection()?;

        let value = conn
            .get(key)
            .map_err(|err| SynxServerError::RedisCMDError(err.to_string()))?;

        match value {
            Value::Data(bytes) => {
                let value = String::from_utf8(bytes)
                    .map_err(|err| SynxServerError::DeserializationError(err.to_string()))?;
                Ok(Some(value))
            }
            _ => Ok(None),
        }
    }

    fn save_to_cache(&self, key: &str, value: &str) -> Result<()> {
        let mut conn = self.get_redis_connection()?;
        conn.set(key, value)
            .map_err(|err| SynxServerError::RedisCMDError(err.to_string()))?;

        Ok(())
    }
}

#[async_trait]
impl Store for StoreV1 {
    async fn get_client_object(&self, id: &str) -> Result<Option<ClientObject>> {
        match self.fetch_from_cache(id)? {
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
        println!("TEST_REDIS_URL: {:?}", std::env::var("TEST_REDIS_URL"));
        let mut store_v1 = StoreV1::new(&DATABASE_URL, &REDIS_URL, &DB_NAME)
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
        store_v1.save_to_cache(key, &value);
        let value = store_v1.fetch_from_cache(key).unwrap();
        let value = serde_json::from_str::<ClientObject>(&value.unwrap()).unwrap();

        assert!(value == client);
    }

    // #[tokio::test]
    // async fn store_test() {
    //   let store_v1 = setup().await;
    //   let
    // }
}