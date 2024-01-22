use super::{
    definitions::{
        R2D2Pool, RedisPool, Result, CACHE_POOL_TIMEOUT_SECONDS, DEFAULT_DIR, JOB_QUEUE,
    },
    errors::SynxServerError,
};
use futures_util::stream::StreamExt;
use r2d2_redis::redis::Commands;
use reqwest;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub struct Queue {
    redis_pool: R2D2Pool,
}

impl RedisPool for Queue {
    fn get_pool(&self) -> &R2D2Pool {
        &self.redis_pool
    }
}

impl Queue {
    pub fn new(redis_pool: R2D2Pool) -> Self {
        Queue { redis_pool }
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
        // Process the job here
        println!("Processing start: {}", job_data);
        // Implement your job processing logic
        match Self::download_file(&job_data).await {
            Ok(file_path) => println!("File {:?} downloaded...", file_path),
            Err(e) => println!("Download of {:?} failed...", job_data),
        };
        println!("Processing end: {}", job_data);
    }

    async fn download_file(gcs_object_name: &str) -> Result<PathBuf> {
        // It's safe to use `unwrap` here
        let bucket_name = std::env::var("GCS_BUCKET_NAME").unwrap();
        let api_key = std::env::var("GOOGLE_STORAGE_API_KEY").unwrap();

        let url = format!(
            "https://storage.googleapis.com/storage/v1/b/{}/o/{}?alt=media",
            bucket_name, gcs_object_name
        );

        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .bearer_auth(api_key)
            .send()
            .await
            .map_err(|_| SynxServerError::DownloadError)?;

        println!("Download response: {:?}", response);

        let parent_dir = Path::new(DEFAULT_DIR);
        let _ = fs::create_dir_all(parent_dir);

        let sub_parent_dir = parent_dir.join("queue");
        let _ = fs::create_dir(&sub_parent_dir);
        let file_path = sub_parent_dir.join(gcs_object_name);

        let mut file = fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&file_path)
            .map_err(|_| SynxServerError::FileOpenError)?;

        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.unwrap();
            file.write_all(&chunk)
                .map_err(|_| SynxServerError::WriteAllError)?;
        }
        Ok(file_path)
    }
}
