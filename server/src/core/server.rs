extern crate proto;

use proto::syncx::{
    syncx_server::Syncx, CreateClientRequest, CreateClientResponse, FileUploadRequest,
    FileUploadResponse,
};
use std::fs;
use std::fs::{read_dir, File};
use std::io::{Read, Write};
use std::path::Path;
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
        println!("Received request: {:?}", request);
        let mut uid = String::new();
        let mut first_chunk = true;

        fs::create_dir_all(DEFAULT_DIR)?;
        let mut file: Option<File> = None;
        let mut stream = request.into_inner();

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
                            fs::File::create(&file_path)
                                .map_err(|err| Status::internal(err.to_string()))?,
                        );
                    }
                    Err(_) => return Err(Status::internal("Authorization failed")),
                };
                first_chunk = false;
            }

            if let Some(ref mut f) = file {
                f.write_all(&chunk.content)?;
            } else {
                // Handle the error: file should have been initialized at this point
                return Err(Status::internal("File not initialized"));
            }
        }

        let response = FileUploadResponse {
            message: "File uploaded successfully".into(),
        };

        Ok(Response::new(response))
    }
}
