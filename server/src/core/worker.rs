use super::{
    definitions::{
        R2D2Pool, RedisPool, Result, CACHE_POOL_TIMEOUT_SECONDS, GCS_PARENT_DIR, JOB_QUEUE,
        MERKLE_DIR, TEMP_DIR, WIP_UPLOADS_DIR,
    },
    errors::SynxServerError,
    utils::*,
};
use common::common::{generate_merkle_tree, list_files_in_dir, unzip_file};
use futures_util::stream::StreamExt;
use log::{debug, error, info};
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use r2d2_redis::redis::Commands;
use reqwest;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub struct Worker {
    redis_pool: Arc<R2D2Pool>,
}

impl RedisPool for Worker {
    fn get_pool(&self) -> &R2D2Pool {
        &self.redis_pool
    }
}

impl Worker {
    pub fn new(redis_pool: Arc<R2D2Pool>) -> Self {
        Worker { redis_pool }
    }

    pub fn dequeue_job(&self) -> Result<String> {
        let mut conn = self.get_redis_connection(CACHE_POOL_TIMEOUT_SECONDS)?;

        let (_, v) = conn
            .blpop::<&str, (String, String)>(JOB_QUEUE, 0)
            .map_err(|err| SynxServerError::DequeueJobError(err.to_string()))?;

        Ok(v)
    }

    fn cache_file_name(&self, key: &str, value: &str) -> Result<()> {
        let mut conn = self.get_redis_connection(CACHE_POOL_TIMEOUT_SECONDS)?;

        let x = conn.set::<&str, &str, String>(key, value).unwrap();

        info!("Saved filename {} to redis.", value);

        Ok(())
    }

    pub async fn run_workers(self: Arc<Self>) {
        info!("Waiting on new jobs in redis queue");

        loop {
            match self.dequeue_job() {
                Ok(job) => {
                    debug!("Processing new job: <{}>", job);
                    let worker_clone = self.clone();
                    tokio::spawn(async move {
                        worker_clone.process_job(job).await;
                    });
                }
                Err(e) => {
                    // TODO: Implement re-try logic
                    error!("Error dequeuing job: {}", e);
                }
            }
        }
    }

    async fn process_job(&self, job_data: String) {
        let id = Self::extract_id_from_job_v(&job_data);

        // It's safe to use `unwrap` here
        let bucket_name = std::env::var("GCS_BUCKET_NAME").unwrap();
        let oauth2_token = std::env::var("GOOGLE_STORAGE_API_KEY").unwrap();

        // Check for the existence of the zip file in the local system directory before proceeding.
        // If the file already exists, it will be reused rather than downloaded again.
        // This approach helps in saving bandwidth and enhances efficiency by avoiding redundant downloads.
        let path = Path::new(&job_data);
        if path.exists() {
            self.unzip_and_upload_all(path, &id, &oauth2_token, &bucket_name)
                .await
                .unwrap();
        } else {
            let file_name = extract_file_name_from_path(Path::new(path)).unwrap();
            let download_path = parse_path_from_slice(&vec![TEMP_DIR, "queued", &file_name]);

            match download_file(&job_data, &bucket_name, &oauth2_token, &download_path).await {
                Ok(()) => {
                    self.unzip_and_upload_all(&download_path, &id, &oauth2_token, &bucket_name)
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
        &self,
        zip_path: &Path,
        id: &str,
        api_key: &str,
        bucket_name: &str,
    ) -> Result<()> {
        let parent_path = Path::new(WIP_UPLOADS_DIR);
        fs::create_dir_all(&parent_path).map_err(|_| SynxServerError::CreateDirectoryError)?;
        let output_path = parent_path.join(id);

        unzip_file(&zip_path, &output_path.as_path()).map_err(|_| SynxServerError::UnzipError)?;

        let mut files_to_upload = list_files_in_dir(&output_path.to_path_buf())
            .map_err(|_| SynxServerError::ListFilesError)?;

        info!("Files to upload: {:?}", files_to_upload);

        // Generate the merkle tree from the files to be uploaded
        let merkle_tree = generate_merkle_tree(&files_to_upload).map_err(|e| {
            error!("Error generating merkle tree: Error {}", e);
            SynxServerError::MerkleTreeGenerationError
        })?;

        let merkle_tree_str = merkle_tree.serialize().map_err(|e| {
            error!("Error serializing merkle tree: Error {}", e);
            SynxServerError::SerializeTreeError
        })?;

        // Write the serialized merkle tree to a file `temp/merkle_trees/{id}.txt`
        let parent_dir = Path::new(MERKLE_DIR);
        fs::create_dir_all(&parent_dir).map_err(|err| {
            error!("Error creating merkle tress temp directory: Error {}", err);
            SynxServerError::CreateDirectoryError
        });

        let merkle_file_path = parent_dir.join("merkletree.txt");

        let mut file = fs::File::create(&merkle_file_path).map_err(|e| {
            error!("Error creating merkle tree file: Error {}", e);
            SynxServerError::CreateFileError
        })?;

        file.write_all(merkle_tree_str.as_bytes()).map_err(|e| {
            error!("Error writing merkle string: Error {}", e);
            SynxServerError::WriteAllError
        })?;

        // Add the merkle tree file to the files to be uploaded
        files_to_upload.push(merkle_file_path);

        let mut count = 0;
        for (i, path) in files_to_upload.iter().enumerate() {
            let file_name = path.as_path().file_name().unwrap().to_string_lossy();
            let object_name = gsc_object_name(&id, &file_name);

            upload_file(&path.as_path(), &id, api_key, bucket_name, &object_name).await?;
            count += 1;

            // We cache the file name to redis for fast lookup. Excluding the "merkletree.txt" file
            if file_name != "merkletree.txt" {
                self.cache_file_name(id, &file_name);
            }
        }
        info!("{} files uploaded", count);
        Ok(())
    }
}
