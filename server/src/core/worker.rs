use super::{
    definitions::{
        R2D2Pool, RedisPool, Result, CACHE_POOL_TIMEOUT_SECONDS, JOB_QUEUE, MERKLE_DIR, TEMP_DIR,
        WIP_UPLOADS_DIR,
    },
    errors::SynxServerError,
    path_resolver::*,
    utils::*,
};
use common::common::{generate_merkle_tree, list_files_in_dir, unzip_file};
use log::{debug, error, info};

use r2d2_redis::redis::Commands;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

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

    fn cache_file_name(&self, key: &str) -> Result<()> {
        let mut conn = self.get_redis_connection(CACHE_POOL_TIMEOUT_SECONDS)?;

        let _x = conn.set::<&str, bool, String>(key, true).unwrap();

        info!("Saved filename {} to redis.", key);

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

    async fn process_job(&self, job_data: String) -> Result<()> {
        let id = &job_data;

        // It's safe to use `unwrap` here
        let bucket_name = std::env::var("GCS_BUCKET_NAME").unwrap();
        let oauth2_token = std::env::var("GOOGLE_STORAGE_API_KEY").unwrap();

        // Check for the existence of the zip file in the local system directory before proceeding.
        // If the file already exists, it will be reused rather than downloaded again.
        // This approach helps in saving bandwidth and enhances efficiency by avoiding redundant downloads.
        let zip_dir = local_zip_dir();
        let path = Path::new(&zip_dir);

        ensure_directory_exists(&path.to_path_buf()).map_err(|err| {
            error!("Error creating local zip path");
            SynxServerError::CreateDirectoryError
        })?;

        let mut zip_file_path = path.join(format!("{}.zip", job_data));
        let wip_uploads_dir = wip_uploads_dir(&job_data);
        let output_zip_dir = Path::new(&wip_uploads_dir);

        ensure_directory_exists(&output_zip_dir.to_path_buf()).map_err(|err| {
            error!("Error creating output zip path");
            SynxServerError::CreateDirectoryError
        })?;

        if zip_file_path.exists() {
            self.unzip_and_upload(
                &zip_file_path,
                &output_zip_dir,
                id,
                &oauth2_token,
                &bucket_name,
            )
            .await
            .map_err(|_| SynxServerError::UnzipError)?
        } else {
            let object_name = gcs_zip_file_object_name(&job_data);
            let output_path_dir = Path::new(&wip_uploads_dir);

            ensure_directory_exists(&output_path_dir.to_path_buf()).map_err(|err| {
                error!("Error creating pending uploads path");
                SynxServerError::CreateDirectoryError
            })?;

            zip_file_path = output_path_dir.join(format!("{}.zip", job_data));

            match download_file(&object_name, &bucket_name, &oauth2_token, &zip_file_path).await {
                Ok(()) => {
                    self.unzip_and_upload(
                        &zip_file_path,
                        &output_zip_dir,
                        &id,
                        &oauth2_token,
                        &bucket_name,
                    )
                    .await
                    .unwrap();
                }
                Err(_) => println!("Download of {:?} failed...", job_data),
            };
        }

        Ok(())
    }

    /// Unzips the contents of a ZIP file.
    ///
    /// This function unzips the file located at `zip_file_path` into the directory specified by `output_path`.
    /// If `output_path` does not exist, it will be created.
    ///
    /// # Arguments
    ///
    /// * `zip_file_path` - A reference to a `Path` that points to the ZIP file to be extracted.
    /// * `output_path` - A reference to a `Path` where the contents of the ZIP file will be extracted to.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the file is successfully unzipped, or `SynxServerError::UnzipError` if the operation fails.
    fn unzip_file(zip_file_path: &Path, output_path: &Path) -> Result<()> {
        let _ = ensure_directory_exists(&output_path.to_path_buf())?;
        unzip_file(&zip_file_path, &output_path).map_err(|_| SynxServerError::UnzipError)?;

        Ok(())
    }

    /// Generates a Merkle tree from a list of files and writes it to a file.
    ///
    /// This function takes an identifier and a list of file paths, generates a Merkle tree from these files,
    /// serializes the Merkle tree, and writes the serialized data to a file.
    ///
    /// # Arguments
    ///
    /// * `id` - A string slice that serves as an identifier for the Merkle tree file.
    /// * `files` - A reference to a vector of `PathBuf`, representing the paths to the files used to generate the Merkle tree.
    ///
    /// # Returns
    ///
    /// Returns `Ok(PathBuf)` containing the path to the created file if the operation is successful.
    /// Returns `Err(SynxServerError)` in case of an error during any of the steps: generating the Merkle tree,
    /// serializing it, creating the output directory, creating the file, or writing to the file.
    ///
    /// #
    fn write_merkle_tree_to_file(id: &str, files: &Vec<PathBuf>) -> Result<PathBuf> {
        let merkle_tree = generate_merkle_tree(files).map_err(|e| {
            error!("Error generating merkle tree: Error {}", e);
            SynxServerError::MerkleTreeGenerationError
        })?;

        // Serialize the merkle tree
        let merkle_tree_str = merkle_tree.serialize().map_err(|e| {
            error!("Error serializing merkle tree: Error {}", e);
            SynxServerError::SerializeTreeError
        })?;

        // Write the serialized merkle tree to a file `temp/merkle_trees/{id}.txt`
        let merkle_tree_path = local_merkle_tree_path();
        let merkle_dir_path = Path::new(&merkle_tree_path);
        let _ = ensure_directory_exists(&merkle_dir_path.to_path_buf()).map_err(|err| {
            error!("Error creating merkle tress temp directory: Error {}", err);
            SynxServerError::CreateDirectoryError
        });

        let merkle_file_path = merkle_dir_path.join(local_merkle_tree_file(id));

        // Create the merkle tree file. The file path is in the format `temp/merkle_trees/{id}_mtree.txt`
        let mut file = fs::File::create(&merkle_file_path).map_err(|e| {
            error!("Error creating merkle tree file: Error {}", e);
            SynxServerError::CreateFileError
        })?;

        file.write_all(merkle_tree_str.as_bytes()).map_err(|e| {
            error!("Error writing merkle string: Error {}", e);
            SynxServerError::WriteAllError
        })?;

        Ok(merkle_file_path)
    }

    async fn unzip_and_upload(
        &self,
        zip_file_path: &Path,
        unzip_output_path: &Path,
        id: &str,
        api_key: &str,
        bucket_name: &str,
    ) -> Result<()> {
        Self::unzip_file(&zip_file_path, &unzip_output_path)
            .map_err(|_| SynxServerError::UnzipError)?;

        let mut files_to_upload = list_files_in_dir(&unzip_output_path.to_path_buf())
            .map_err(|_| SynxServerError::ListFilesError)?;

        info!("Files to upload: {:?}", files_to_upload);

        let merkle_file_path = Self::write_merkle_tree_to_file(id, &files_to_upload)?;

        // Add the merkle tree file to the files to be uploaded
        files_to_upload.push(merkle_file_path);

        let mut count = 0;
        for (_i, path) in files_to_upload.iter().enumerate() {
            let file_name = path.as_path().file_name().unwrap().to_string_lossy();
            let object_name = gsc_object_name(&id, &file_name);

            upload_file(&path.as_path(), &id, api_key, bucket_name, &object_name).await?;
            count += 1;

            // We cache the file name to redis for fast lookup. Excluding the "merkletree.txt" file
            if file_name != "merkletree.txt" {
                let key = hash_str(&format!("{}{}", id, file_name));
                let _ = self.cache_file_name(&key);
            }
        }
        info!("{} files uploaded", count);
        Ok(())
    }
}
