use super::{
    definitions::{
        R2D2Pool, RedisPool, Result, CACHE_POOL_TIMEOUT_SECONDS, DEFAULT_DIR, JOB_QUEUE,
    },
    errors::SynxServerError,
    utils::extract_file_name_from_path,
};
use futures_util::stream::StreamExt;
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use r2d2_redis::redis::Commands;
use reqwest;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct Worker {
    redis_pool: R2D2Pool,
}

impl RedisPool for Worker {
    fn get_pool(&self) -> &R2D2Pool {
        &self.redis_pool
    }
}

impl Worker {
    pub fn new(redis_pool: R2D2Pool) -> Self {
        Worker { redis_pool }
    }

    pub fn dequeue_job(&self) -> Result<String> {
        let mut conn = self.get_redis_connection(CACHE_POOL_TIMEOUT_SECONDS)?;

        let (_, v) = conn
            .blpop::<&str, (String, String)>(JOB_QUEUE, 0)
            .map_err(|err| SynxServerError::DequeueJobError(err.to_string()))?;

        Ok(v)
    }

    pub async fn run_workers(&self) {
        println!("Running workers...");

        loop {
            match self.dequeue_job() {
                Ok(job) => {
                    println!("New job: {:?}", job);
                    tokio::spawn(async move {
                        Self::process_job(job).await;
                    });
                    println!("After new job");
                }
                Err(e) => {
                    // TODO: Implement re-try logic
                    eprintln!("Error dequeuing job: {}", e);
                }
            }
        }
    }

    async fn process_job(job_data: String) {
        match Self::download_file(&job_data).await {
            Ok(file_path) => println!("File {:?} downloaded...", file_path),
            Err(_) => println!("Download of {:?} failed...", job_data),
        };
    }

    async fn download_file(object_name: &str) -> Result<PathBuf> {
        // It's safe to use `unwrap` here
        let bucket_name = std::env::var("GCS_BUCKET_NAME").unwrap();
        let oauth2_token = std::env::var("GOOGLE_STORAGE_API_KEY").unwrap();

        const FRAGMENT: &AsciiSet = &CONTROLS.add(b'/');
        let gcs_object_name = utf8_percent_encode(&object_name, FRAGMENT).to_string();

        let mut url = format!(
            "https://storage.googleapis.com/storage/v1/b/{}/o/{}?alt=media",
            bucket_name, gcs_object_name
        );

        let client = reqwest::Client::new();
        let response = client
            .get(&url)
            .bearer_auth(oauth2_token)
            .send()
            .await
            .map_err(|_| SynxServerError::DownloadError)?;

        let body = response
            .bytes()
            .await
            .map_err(|_| SynxServerError::HttpReadBytesError)?;

        let parent_dir = Path::new(DEFAULT_DIR);
        let _ = fs::create_dir_all(parent_dir);

        let sub_parent_dir = parent_dir.join("queued");
        let _ = fs::create_dir(&sub_parent_dir);

        let file_path =
            sub_parent_dir.join(extract_file_name_from_path(Path::new(object_name)).unwrap());

        let mut file = fs::File::create(&file_path).unwrap();
        file.write_all(&body)
            .map_err(|_| SynxServerError::FileOpenError)?;

        Ok(file_path)
    }
}
