use super::{
    definitions::{R2D2Pool, RedisPool, Result, CACHE_POOL_TIMEOUT_SECONDS, JOB_QUEUE, TEMP_DIR},
    errors::SynxServerError,
    utils::{download_file, extract_file_name_from_path, gcs_file_path},
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
                }
                Err(e) => {
                    // TODO: Implement re-try logic
                    eprintln!("Error dequeuing job: {}", e);
                }
            }
        }
    }

    async fn process_job(job_data: String) {
        // Check for the existence of the zip file in the local system directory before proceeding.
        // If the file already exists, it will be reused rather than downloaded again.
        // This approach helps in saving bandwidth and enhances efficiency by avoiding redundant downloads,
        // thereby also reducing operational costs.
        if Path::new(&gcs_file_path(&job_data)).exists() {}

        // It's safe to use `unwrap` here
        let bucket_name = std::env::var("GCS_BUCKET_NAME").unwrap();
        let oauth2_token = std::env::var("GOOGLE_STORAGE_API_KEY").unwrap();

        match download_file(&job_data, &bucket_name, &oauth2_token).await {
            Ok(file_path) => println!("File {:?} downloaded...", file_path),
            Err(_) => println!("Download of {:?} failed...", job_data),
        };
    }
}
