extern crate common;

use common::syncx::{
    syncx_server::Syncx, CreateClientRequest, CreateClientResponse, FileUploadRequest,
    FileUploadResponse,
};

use reqwest;

use std::fs;
use std::fs::File;
use std::io::Write;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tonic::{Request, Response, Status};
use uuid::Uuid;

use super::errors::SynxServerError;
use super::{
    auth,
    config::Config,
    definitions::{ClientObject, Result, Store, DEFAULT_DIR, DEFAULT_ZIP_FILE, TEMP_DIR},
};

// #[derive(Debug, Clone)]
pub struct Server<T> {
    store: T,
    config: Config,
    http_client: reqwest::Client,
}

impl<T> Server<T> {
    pub async fn new(store: T, config: Config) -> Self
    where
        T: Store + Send + Sync + 'static,
    {
        Self {
            store,
            config,
            http_client: reqwest::Client::new(),
        }
    }

    async fn upload_file(&self, file_path: &Path, uid: &str) -> Result<()> {
        let file_contents = fs::read(file_path).map_err(|_| SynxServerError::ReadFileError)?;
        let object_name = Self::gcs_file_path(&uid);

        let url = format!(
            "https://storage.googleapis.com/upload/storage/v1/b/{}/o?uploadType=media&name={}",
            self.config.gcs_bucket_name, object_name
        );

        let api_key = std::env::var("GOOGLE_STORAGE_API_KEY").unwrap();

        let _response = self
            .http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .body(reqwest::Body::from(std::fs::read(file_path).map_err(
                |err| SynxServerError::UploadFileRequestError(err.to_string()),
            )?))
            .send()
            .await
            .map_err(|err| SynxServerError::UploadFileRequestError(err.to_string()))?;

        println!("Fil uploaded successfully {:?}", _response);

        Ok(())
    }

    pub fn gcs_file_path(id: &str) -> (String) {
        format!("{}/{}.zip", TEMP_DIR, id)
    }
}

#[tonic::async_trait]
impl<T> Syncx for Server<T>
where
    T: Store + Send + Sync + 'static,
{
    async fn register_client(
        &self,
        request: Request<CreateClientRequest>,
    ) -> std::result::Result<Response<CreateClientResponse>, Status> {
        let id = (Uuid::new_v4()).to_string();

        let jwt_token = auth::jwt::create_jwt(&id, &self.config.jwt_secret, self.config.jwt_exp)
            .map_err(|_| Status::internal("Failed to create auth token"))?;

        let password = request.into_inner().password;
        let hashed_password = auth::hash_utils::hash_password(&password)
            .map_err(|_| Status::internal("Failed to hash password"))?;

        let client_object = ClientObject {
            id: id.clone(),
            password: hashed_password,
        };

        self.store
            .save_client_object(client_object)
            .await
            .map_err(|_| Status::internal("Failed to save client object"))?;

        let response = CreateClientResponse { id, jwt_token };

        Ok(Response::new(response))
    }

    async fn upload_files(
        &self,
        request: tonic::Request<tonic::Streaming<FileUploadRequest>>,
    ) -> std::result::Result<Response<FileUploadResponse>, Status> {
        let mut uid = String::new();
        let mut first_chunk = true;

        fs::create_dir_all(DEFAULT_DIR)?;

        let mut file: Option<File> = None;
        let mut zip_path: Option<PathBuf> = None;

        let mut stream = request.into_inner();
        let mut all_chunks: Vec<u8> = Vec::new();
        while let Some(chunk) = stream.message().await? {
            if first_chunk {
                match auth::jwt::verify_jwt(&chunk.jwt, &self.config.jwt_secret) {
                    Ok(claims) => {
                        uid = claims.sub;
                        // Create the outer directory if it doesn't exist
                        let outer_dir = Path::new(DEFAULT_DIR);
                        fs::create_dir_all(&outer_dir)?;

                        // Create the inner directory within the outer directory
                        let inner_dir = outer_dir.join(&uid);
                        fs::create_dir(&inner_dir)?;

                        let file_path = inner_dir.join(DEFAULT_ZIP_FILE);
                        file = Some(
                            fs::OpenOptions::new()
                                .append(true)
                                .create(true)
                                .open(&file_path)?,
                        );
                        zip_path = Some(file_path);
                    }
                    Err(_) => return Err(Status::internal("Authorization failed")),
                };
                first_chunk = false;
            }

            if let Some(ref mut f) = file {
                f.write_all(&chunk.content)?;
                all_chunks.extend(&chunk.content);
            } else {
                // Handle the error: file should have been initialized at this point
                return Err(Status::internal("File not initialized"));
            }
        }

        let response = FileUploadResponse {
            message: "File uploaded successfully".into(),
        };

        self.upload_file(&zip_path.unwrap(), &uid).await.unwrap();

        let value = Self::gcs_file_path(&uid);
        let _ = self.store.enqueue_job(&value);

        Ok(Response::new(response))
    }
}
