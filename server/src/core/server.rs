extern crate common;

use common::syncx::{
    syncx_server::Syncx, CreateClientRequest, CreateClientResponse, FileUploadRequest,
    FileUploadResponse,
};

use reqwest;

use log::{debug, error, info};
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use tonic::{Request, Response, Status};
use uuid::Uuid;

use super::errors::SynxServerError;
use super::{
    auth,
    config::Config,
    definitions::{ClientObject, Result, Store, DEFAULT_ZIP_FILE, TEMP_DIR},
    utils::{gcs_file_path, upload_file},
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
        info!("New request to register client");

        let id = (Uuid::new_v4()).to_string();

        debug!("Generating JWT for account #{}", id);

        let jwt_token = auth::jwt::create_jwt(&id, &self.config.jwt_secret, self.config.jwt_exp)
            .map_err(|e| {
                error!("Failed to generate JWT for account #{}. Error {}", id, e);
                Status::internal("Failed to create auth token")
            })?;

        let password = request.into_inner().password;
        let hashed_password = auth::hash_utils::hash_password(&password).map_err(|e| {
            error!("Error hashing password for account #{}. Error {}", id, e);
            Status::internal("Failed to hash password")
        })?;

        let client_object = ClientObject {
            id: id.clone(),
            password: hashed_password,
        };

        self.store
            .save_client_object(client_object)
            .await
            .map_err(|e| {
                error!("Error saving client object #{}. Error {}", id, e);
                Status::internal("Failed to save client object")
            })?;

        debug!("New client #{} created", &id);

        let response = CreateClientResponse { id, jwt_token };

        Ok(Response::new(response))
    }

    async fn upload_files(
        &self,
        request: tonic::Request<tonic::Streaming<FileUploadRequest>>,
    ) -> std::result::Result<Response<FileUploadResponse>, Status> {
        let mut uid = String::new();
        let mut first_chunk = true;
        info!("New client #{}request to upload files", uid);

        // Create the outer directory if it doesn't exist
        let parent_dir = Path::new(TEMP_DIR);
        fs::create_dir_all(&parent_dir)?;

        let mut file: Option<File> = None;
        let mut zip_path: Option<PathBuf> = None;

        let mut stream = request.into_inner();
        let mut all_chunks: Vec<u8> = Vec::new();

        info!("Streaming and recreating file {}.zip", uid);

        while let Some(chunk) = stream.message().await? {
            if first_chunk {
                match auth::jwt::verify_jwt(&chunk.jwt, &self.config.jwt_secret) {
                    Ok(claims) => {
                        uid = claims.sub;

                        // Create the inner directory within the outer directory
                        let file_path = parent_dir.join(format!("{}.zip", uid));

                        file = Some(
                            fs::OpenOptions::new()
                                .append(true)
                                .create(true)
                                .open(&file_path)?,
                        );
                        zip_path = Some(file_path);

                        debug!("Zip file created {}/{}.zip", TEMP_DIR, uid);
                    }
                    Err(_) => {
                        error!("Un-authorized access with JWT {}", &chunk.jwt);
                        return Err(Status::internal("Authorization failed"));
                    }
                };
                first_chunk = false;
            }

            if let Some(ref mut f) = file {
                f.write_all(&chunk.content)?;
                all_chunks.extend(&chunk.content);
            } else {
                return Err(Status::internal("File not initialized"));
            }
        }

        let response = FileUploadResponse {
            message: "File uploaded successfully".into(),
        };

        let api_key = std::env::var("GOOGLE_STORAGE_API_KEY").unwrap();
        upload_file(
            &zip_path.unwrap(),
            &uid,
            &api_key,
            &self.config.gcs_bucket_name,
            &gcs_file_path(&uid),
        )
        .await
        .unwrap();

        let value = gcs_file_path(&uid);
        let _ = self.store.enqueue_job(&value);
        info!("New job <{}> queued", value);

        Ok(Response::new(response))
    }
}
