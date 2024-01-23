use super::{
    definitions::{
        R2D2Pool, Result, CACHE_POOL_EXPIRE_SECONDS, CACHE_POOL_MAX_OPEN, CACHE_POOL_MIN_IDLE,
        GCS_PARENT_DIR, TEMP_DIR,
    },
    errors::SynxServerError,
};

use hex;
use log::{debug, error, info};
use mongodb::{options::ClientOptions, Client};
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use r2d2_redis::{r2d2, RedisConnectionManager};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

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
    let client_options = ClientOptions::parse(db_url).await.map_err(|err| {
        error!("Error connecting to database: Error {}", err);
        SynxServerError::DatabaseConnectionError(err.to_string())
    })?;

    let client = Client::with_options(client_options).map_err(|err| {
        error!("Error creating database client: Error {}", err);
        SynxServerError::DbOptionsConfigurationError(err.to_string())
    })?;

    Ok(client)
}

pub fn connect_redis(url: &str) -> Result<R2D2Pool> {
    let manager = RedisConnectionManager::new(url).map_err(|err| {
        error!("Error connecting to redis: Error {}", err);
        SynxServerError::RedisConnectionError(err.to_string())
    })?;

    let pool_manager = r2d2::Pool::builder()
        .max_size(CACHE_POOL_MAX_OPEN)
        .max_lifetime(Some(Duration::from_secs(CACHE_POOL_EXPIRE_SECONDS)))
        .min_idle(Some(CACHE_POOL_MIN_IDLE))
        .build(manager)
        .map_err(|err| {
            error!("Error creating redis pool: Error {}", err);
            SynxServerError::RedisConnectionError(err.to_string())
        })?;

    Ok(pool_manager)
}

pub async fn download_file(
    object_name: &str,
    gcs_bucket_name: &str,
    api_key: &str,
    file_path: &Path,
) -> Result<()> {
    info!("Attempting to download file {:?} from storage", object_name);

    const FRAGMENT: &AsciiSet = &CONTROLS.add(b'/');
    let gcs_object_name = utf8_percent_encode(&object_name, FRAGMENT).to_string();

    let url = format!(
        "https://storage.googleapis.com/storage/v1/b/{}/o/{}?alt=media",
        gcs_bucket_name, gcs_object_name
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .bearer_auth(api_key)
        .send()
        .await
        .map_err(|e| {
            error!("File download failed: Error {}", e);
            SynxServerError::DownloadError
        })?;

    debug!(
        "File {:?} downloaded successfully with status_code {}",
        gcs_object_name,
        response.status()
    );

    let body = &response.bytes().await.map_err(|e| {
        error!("Error reading downloaded bytes: Error {}", e);
        SynxServerError::HttpReadBytesError
    })?;

    let mut file = fs::File::create(&file_path).unwrap();
    file.write_all(&body).map_err(|e| {
        error!("Error creating file from downloaded bytes: Error {}", e);
        SynxServerError::FileOpenError
    })?;

    Ok(())
}

pub async fn upload_file(
    file_path: &Path,
    id: &str,
    api_key: &str,
    gcs_bucket_name: &str,
    object_name: &str,
) -> Result<()> {
    info!("Attempting to upload file {:?}", file_path);

    let url = format!(
        "https://storage.googleapis.com/upload/storage/v1/b/{}/o?uploadType=media&name={}",
        gcs_bucket_name, object_name
    );

    let client = reqwest::Client::new();
    let response = client
        .post(&url)
        .bearer_auth(api_key)
        .body(reqwest::Body::from(std::fs::read(file_path).map_err(
            |err| {
                error!("File upload failed: Error {}", err);
                SynxServerError::UploadFileRequestError(err.to_string())
            },
        )?))
        .send()
        .await
        .map_err(|err| {
            error!("File upload failed: Error {}", err);
            SynxServerError::UploadFileRequestError(err.to_string())
        })?;

    info!(
        "File {:?} uploaded successfully with status_code {}",
        file_path,
        response.status()
    );

    Ok(())
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

pub fn gcs_zip_path(id: &str) -> String {
    format!("{}/{}.zip", TEMP_DIR, id)
}

pub fn gsc_object_name(id: &str, file_name: &str) -> String {
    format!("{}/{}/{}", GCS_PARENT_DIR, id, file_name)
}

pub fn parse_path_from_slice(tokens: &Vec<&str>) -> PathBuf {
    let path_str = tokens.iter().map(|s| s.to_string()).collect::<String>();
    Path::new(&path_str).to_path_buf()
}

pub fn hash_str(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value);
    hex::encode(hasher.finalize())
}

pub fn ensure_directory_exists(path: &PathBuf) -> Result<()> {
    if !path.exists() {
        fs::create_dir_all(path).map_err(|_| SynxServerError::CreateDirectoryError)?;
    }

    Ok(())
}
