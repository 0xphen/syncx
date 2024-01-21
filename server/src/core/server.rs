extern crate common;

use common::common::*;
use common::syncx::{
    syncx_server::Syncx, CreateClientRequest, CreateClientResponse, FileUploadRequest,
    FileUploadResponse,
};

use google_cloud_auth::credentials::CredentialsFile;
use google_cloud_storage::client::{Client, ClientConfig};
use google_cloud_storage::http::objects::upload::{Media, UploadObjectRequest, UploadType};

use reqwest;

use merkle_tree::utils::hash_bytes;
use std::fs;
use std::fs::{read_dir, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use super::{auth, config::Config, definitions::ClientObject, definitions::Store};

type UploadFileStream = ReceiverStream<Result<FileUploadResponse, Status>>;

const DEFAULT_DIR: &str = "temp";
const DEFAULT_ZIP_FILE: &str = "uploads.zip";

pub struct Server<T> {
    pub store: T,
    pub config: Config,
}

impl<T> Server<T> {
    pub async fn new(store: T, config: Config) -> Self
    where
        T: Store + Send + Sync + 'static,
    {
        Self { store, config }
    }

    async fn upload_files(zip_path: &Path) {
        let bucket_name = "syncx_bucket";
        let object_name = "test_object";
        let api_key = std::env::var("GOOGLE_STORAGE_API_KEY").unwrap();

        let parent_folder = zip_path.parent().unwrap();
        unzip_file(zip_path, parent_folder).unwrap();
        // println!("SEE: {:?} {:?}", zip_file, parent_folder);

        // Build the API URL.
        // let api_url = format!(
        //     "https://storage.googleapis.com/upload/storage/v1/b/{}/o?uploadType=media&name={}",
        //     bucket_name, "temp%2Ftest.txt"
        // );

        // // Create a client and send a GET request to the API.
        // let client = reqwest::Client::new();
        // let response = client
        //     .get(&api_url)
        //     .header("Authorization", format!("Bearer {}", api_key))
        //     .header("Content-Type", "application/json")
        //     .body(reqwest::Body::from(file_contents))
        //     .send()
        //     .await;

        // println!("response: {:?}", response);
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
    ) -> Result<Response<CreateClientResponse>, Status> {
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
    ) -> Result<Response<FileUploadResponse>, Status> {
        let mut uid = String::new();
        let mut first_chunk = true;
        println!("Client checksum:{:?}", request.metadata().get("checksum"));
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
                        // file = Some(
                        //     fs::File::create(&file_path)
                        //         .map_err(|err| Status::internal(err.to_string()))?,
                        // );
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

        println!("Server checksum: {:?}", hash_bytes(&all_chunks));
        Self::upload_files(&zip_path.clone().unwrap()).await;
        Ok(Response::new(response))
    }
}
