use super::{
    definitions::{
        R2D2Pool, RedisPool, Result, CACHE_POOL_TIMEOUT_SECONDS, GCS_PARENT_DIR, JOB_QUEUE,
        PENDING_UPLOADS_DIR, TEMP_DIR,
    },
    errors::SynxServerError,
    utils::{download_file, extract_file_name_from_path, gcs_file_path, upload_file},
};
use common::common::{generate_merkle_tree, list_files_in_dir, unzip_file};
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
        let id = Self::extract_id_from_job_v(&job_data);

        // It's safe to use `unwrap` here
        let bucket_name = std::env::var("GCS_BUCKET_NAME").unwrap();
        let oauth2_token = std::env::var("GOOGLE_STORAGE_API_KEY").unwrap();

        // Check for the existence of the zip file in the local system directory before proceeding.
        // If the file already exists, it will be reused rather than downloaded again.
        // This approach helps in saving bandwidth and enhances efficiency by avoiding redundant downloads.
        let path = Path::new(&job_data);
        if path.exists() {
            println!("Exists...: {}", id);
            Self::unzip_and_upload_all(path, &id, &oauth2_token, &bucket_name)
                .await
                .unwrap();
        } else {
            print!("Not exists...: {}", id);
            match download_file(&job_data, &bucket_name, &oauth2_token).await {
                Ok(file_path) => {
                    println!("File {:?} downloaded...", file_path);
                    Self::unzip_and_upload_all(&file_path, &id, &oauth2_token, &bucket_name)
                        .await
                        .unwrap();
                }
                Err(_) => println!("Download of {:?} failed...", job_data),
            };
        }
    }

    // Extracts the an ID from a job value string. The job value is expected to be in the format
    // "temp/id.zip", where `id` represents the unique identifier of the job and an account ID.
    fn extract_id_from_job_v(job_data: &str) -> String {
        let mut parts = job_data.split(".").collect::<Vec<&str>>();
        parts = parts[0].split("/").collect::<Vec<&str>>();
        parts[1].to_string()
    }

    async fn unzip_and_upload_all(
        zip_path: &Path,
        id: &str,
        api_key: &str,
        bucket_name: &str,
    ) -> Result<()> {
        let parent_path = Path::new(PENDING_UPLOADS_DIR);
        fs::create_dir_all(&parent_path).map_err(|_| SynxServerError::CreateDirectoryError)?;
        let output_path = parent_path.join(id);

        unzip_file(&zip_path, &output_path.as_path()).map_err(|_| SynxServerError::UnzipError)?;

        let files_to_upload = list_files_in_dir(&output_path.to_path_buf())
            .map_err(|_| SynxServerError::ListFilesError)?;
        // Generate the merkle tree from the files to be uploaded
        let merkle_tree = generate_merkle_tree(&files_to_upload)
            .map_err(|_| SynxServerError::MerkleTreeGenerationError)?;

        for (i, path) in files_to_upload.iter().enumerate() {
            let file_name = path.as_path().file_name().unwrap().to_string_lossy();
            let object_name = format!("{}/{}/{}", GCS_PARENT_DIR, id, file_name);

            upload_file(&path.as_path(), &id, api_key, bucket_name, &object_name).await?;
        }

        println!("File upload successfull...");
        Ok(())
    }
}
