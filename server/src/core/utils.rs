use super::{
    definitions::{
        CACHE_POOL_EXPIRE_SECONDS, CACHE_POOL_MAX_OPEN, CACHE_POOL_MIN_IDLE, QUEUE_DIR, TEMP_DIR,
    },
    errors::SynxServerError,
};

use mongodb::{options::ClientOptions, Client};
use r2d2_redis::{r2d2, RedisConnectionManager};
use std::fs;
use std::path::Path;
use std::time::Duration;

use super::definitions::{R2D2Pool, Result};

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
pub async fn connect_db(db_url: &str) -> Result<Client> {
    let client_options = ClientOptions::parse(db_url)
        .await
        .map_err(|err| SynxServerError::DatabaseConnectionError(err.to_string()))?;

    let client = Client::with_options(client_options)
        .map_err(|err| SynxServerError::DbOptionsConfigurationError(err.to_string()))?;

    Ok(client)
}

pub fn connect_redis(url: &str) -> Result<R2D2Pool> {
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

pub fn extract_file_name_from_path(path: &Path) -> Option<String> {
    if let Some(file_name) = path.file_name() {
        Some(file_name.to_str().unwrap().to_string())
    } else {
        None
    }
}

pub fn delete_file_or_dir(path: &Path) -> std::io::Result<()> {
    if path.is_dir() {
        // Remove a directory and all its contents
        fs::remove_dir_all(path)
    } else if path.is_file() {
        // Remove a file
        fs::remove_file(path)
    } else {
        // Path does not exist or is neither a file nor a directory
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Path is not a file or directory",
        ))
    }
}

pub fn gcs_file_path(id: &str) -> String {
    format!("{}/{}.zip", TEMP_DIR, id)
}
