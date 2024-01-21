use super::errors::SynxServerError;

use async_trait::async_trait;
use r2d2_redis::{r2d2, RedisConnectionManager};
use serde::{Deserialize, Serialize};
use std::time::Duration;

pub type Result<T> = std::result::Result<T, SynxServerError>;
pub type R2D2Pool = r2d2::Pool<RedisConnectionManager>;
pub type R2D2Con = r2d2::PooledConnection<RedisConnectionManager>;

pub const CACHE_POOL_TIMEOUT_SECONDS: u64 = 1;
pub const DEFAULT_DIR: &str = "temp";
pub const TEMP_DIR: &str = "temp";
pub const DEFAULT_ZIP_FILE: &str = "uploads.zip";
pub const JOB_QUEUE: &str = "syncx_queue";

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ClientObject {
    pub id: String,
    pub password: String,
}

#[async_trait]
pub trait Store {
    async fn get_client_object(&self, id: &str) -> Result<Option<ClientObject>>;

    async fn save_client_object(&self, client_object: ClientObject) -> Result<bool>;

    fn enqueue_job(&self, value: &str) -> Result<()>;
}

pub trait RedisPool {
    fn get_pool(&self) -> &R2D2Pool;

    fn get_redis_connection(&self, timeout: u64) -> Result<R2D2Con> {
        self.get_pool()
            .get_timeout(Duration::from_secs(timeout))
            .map_err(|e| {
                eprintln!("error connecting to redis: {}", e);
                SynxServerError::RedisPoolError(e.to_string())
            })
    }
}
