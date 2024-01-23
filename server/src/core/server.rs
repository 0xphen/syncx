extern crate common;

use common::{
    common::{file_to_bytes, generate_merkle_tree},
    syncx::{
        syncx_server::Syncx, CreateClientRequest, CreateClientResponse, FileDownloadRequest,
        FileDownloadResponse, FileUploadRequest, FileUploadResponse, MerkleProof, MerkleProofNode,
    },
};
use merkle_tree::{merkle_tree::MerkleTree, utils::hash_bytes};
use rayon::prelude::*;
use reqwest;

use log::{debug, error, info};
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use super::{
    auth,
    config::Config,
    definitions::{ClientObject, Result, Store, TEMP_DIR, WIP_DOWNLOADS_DIR},
    errors::SynxServerError,
    utils::*,
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
    type DownloadFileStream = ReceiverStream<std::result::Result<FileDownloadResponse, Status>>;

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
        // let mut all_chunks: Vec<u8> = Vec::new();

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
                // all_chunks.extend(&chunk.content);
            } else {
                return Err(Status::internal("File not initialized"));
            }
        }

        let response = FileUploadResponse {
            message: "File uploaded successfully".into(),
        };

        upload_file(
            &zip_path.unwrap(),
            &uid,
            &self.config.api_key,
            &self.config.gcs_bucket_name,
            &gcs_zip_path(&uid),
        )
        .await
        .unwrap();

        let value = gcs_zip_path(&uid);
        let _ = self.store.enqueue_job(&value);
        info!("New job <{}> queued", value);

        Ok(Response::new(response))
    }

    async fn download_file(
        &self,
        request: tonic::Request<FileDownloadRequest>,
    ) -> std::result::Result<Response<Self::DownloadFileStream>, Status> {
        let FileDownloadRequest { jwt, file_name } = request.into_inner();
        match auth::jwt::verify_jwt(&jwt, &self.config.jwt_secret) {
            Ok(claims) => {
                let value = self
                    .store
                    .fetch_from_cache(&hash_str(&format!("{}{}", &claims.sub, &file_name)))
                    .map_err(|err| {
                        error!("Error getting value of key {} from redis", &file_name);
                        Status::internal("Internal server error")
                    })?;

                // If file does not exists in cache, it means user has not uploaded such file.
                if value.is_none() {
                    println!("Not found in cache: {:?}", value);
                    return Err(Status::internal(format!("File {} not found", file_name)));
                }

                let download_path = parse_path_from_slice(&vec![WIP_DOWNLOADS_DIR, &claims.sub]);

                let download_path = Path::new(download_path.as_path());
                fs::create_dir_all(&download_path)?;

                let file_1 = gsc_object_name(&claims.sub, &file_name);
                let path_1 = download_path.join(&file_name);

                let file_2 = gsc_object_name(&claims.sub, "merkletree.txt");
                let path_2 = download_path.join("merkletree.txt");

                let files_and_download_path = vec![(file_1, path_1), (file_2, path_2)];

                for (name, path) in &files_and_download_path {
                    let f = download_file(
                        &name,
                        &self.config.gcs_bucket_name,
                        &self.config.api_key,
                        path.as_path(),
                    )
                    .await
                    .map_err(|err| {
                        error!("Error downloading file {}. Err: {}", file_name, err);
                        Status::internal("Internal server error")
                    })?;
                }

                let content = file_to_bytes(&files_and_download_path[0].1).map_err(|e| {
                    error!("Error converting file to bytes. Error {}", e);
                    Status::internal("Internal server error")
                })?;

                let merkle_tree_bytes = file_to_bytes(&files_and_download_path[1].1)
                    .map_err(|_| Status::internal("Internal server error"))?;

                let merkle_tree = MerkleTree::from_bytes(&merkle_tree_bytes)
                    .map_err(|_| Status::internal("Internal server error"))?;

                let merkle_proof = merkle_tree
                    .generate_merkle_proof(&hash_bytes(&content))
                    .map_err(|_| {
                        Status::internal("Internal server error. Error generating merkle proof")
                    })?;
               
                let merkle_proof_nodes = merkle_proof
                    .into_iter()
                    .map(|(hash, flag)| MerkleProofNode {
                        hash,
                        flag: flag.into(),
                    })
                    .collect::<Vec<MerkleProofNode>>();

                let merkle_proof = Some(MerkleProof {
                    nodes: merkle_proof_nodes,
                });

                let (mut tx, rx) = mpsc::channel(4);
                // Here, spawn a new task to handle file reading and streaming
                tokio::spawn(async move {
                    let chunk = FileDownloadResponse {
                        content,
                        merkle_proof,
                    };

                    tx.send(Ok(chunk)).await.map_err(|err| {
                        error!("Error streaming chunk to client: Error {}", err);
                        Status::internal("Internal server error")
                    });
                });

                Ok(Response::new(Self::DownloadFileStream::new(rx)))
            }
            Err(_) => {
                error!("Un-authorized access with JWT {}", &jwt);
                Err(Status::internal("Authorization failed"))
            }
        }
    }
}
