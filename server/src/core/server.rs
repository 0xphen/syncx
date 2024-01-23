extern crate common;

use common::{
    common::file_to_bytes,
    syncx::{
        syncx_server::Syncx, CreateClientRequest, CreateClientResponse, FileDownloadRequest,
        FileDownloadResponse, FileUploadRequest, FileUploadResponse, MerkleProof, MerkleProofNode,
    },
};
use merkle_tree::{merkle_tree::MerkleTree, utils::hash_bytes};

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
    path_resolver::*,
    utils::*,
};

#[derive(Debug)]
pub struct Server<T> {
    store: T,
    config: Config,
}

impl<T: Store> Server<T> {
    pub async fn new(store: T, config: Config) -> Self
    where
        T: Store + Send + Sync + 'static,
    {
        Self { store, config }
    }

    fn file_exists(&self, file_name: &str, id: &str) -> Result<Option<String>> {
        Ok(self
            .store
            .fetch_from_cache(&hash_str(&format!("{}{}", id, &file_name)))
            .map_err(|e| SynxServerError::RedisCMDError(e.to_string()))?)
    }

    async fn download_file(&self, id: &str, file_name: &str) -> Result<Vec<(String, PathBuf)>> {
        let wip_dir = wip_downloads_dir(id);
        let download_path = Path::new(&wip_dir);

        let _ = ensure_directory_exists(&download_path.to_path_buf())?;

        let obj_name_1 = gcs_backup_object_name(id, file_name);
        let path_1 = download_path.join(&file_name);

        let merkle_file_name = local_merkle_tree_file(id);
        let obj_name_2 = gcs_backup_object_name(id, &merkle_file_name);
        let path_2 = download_path.join("merkletree.txt");

        let files_and_download_path = vec![(obj_name_1, path_1), (obj_name_2, path_2)];

        for (name, path) in &files_and_download_path {
            let _ = download_file(
                &name,
                &self.config.gcs_bucket_name,
                &self.config.api_key,
                path.as_path(),
            )
            .await?;
        }

        Ok(files_and_download_path)
    }

    fn generate_merkle_proof(
        &self,
        files_and_download_path: Vec<(String, PathBuf)>,
        download_bytes: &Vec<u8>,
    ) -> Result<Vec<MerkleProofNode>> {
        let merkle_tree_bytes = file_to_bytes(&files_and_download_path[1].1)
            .map_err(|_| SynxServerError::ConvertFileToBytesError)?;

        let merkle_tree = MerkleTree::from_bytes(&merkle_tree_bytes)
            .map_err(|_| SynxServerError::MerkleTreeGenerationError)?;

        let merkle_proof = merkle_tree
            .generate_merkle_proof(&hash_bytes(&download_bytes))
            .map_err(|_| SynxServerError::MerkleTreeGenerationError)?;

        Ok(merkle_proof
            .into_iter()
            .map(|(hash, flag)| MerkleProofNode {
                hash,
                flag: flag.into(),
            })
            .collect::<Vec<MerkleProofNode>>())
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

        let local_zip_dir = local_zip_dir();
        let zip_dir = Path::new(&local_zip_dir);
        let _ = ensure_directory_exists(&zip_dir.to_path_buf()).map_err(|_| {
            error!("Error creating local zip dir");
            Status::internal("Authorization failed")
        });

        let mut file: Option<File> = None;
        let mut zip_path: Option<PathBuf> = None;

        let mut stream = request.into_inner();
        info!("Streaming and recreating file {}.zip", uid);

        while let Some(chunk) = stream.message().await? {
            if first_chunk {
                match auth::jwt::verify_jwt(&chunk.jwt, &self.config.jwt_secret) {
                    Ok(claims) => {
                        uid = claims.sub;

                        // Create the inner directory within the outer directory
                        let file_path = zip_dir.join(format!("{}.zip", uid));

                        file = Some(
                            fs::OpenOptions::new()
                                .append(true)
                                .create(true)
                                .open(&file_path)?,
                        );

                        debug!("Zip file created {:?}", file_path);

                        zip_path = Some(file_path);
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
            &gcs_zip_file_object_name(&uid),
        )
        .await
        .unwrap();

        let _ = self.store.enqueue_job(&uid);
        info!("New job <{}> queued", uid);

        Ok(Response::new(response))
    }

    async fn download_file(
        &self,
        request: tonic::Request<FileDownloadRequest>,
    ) -> std::result::Result<Response<Self::DownloadFileStream>, Status> {
        let FileDownloadRequest { jwt, file_name } = request.into_inner();

        match auth::jwt::verify_jwt(&jwt, &self.config.jwt_secret) {
            Ok(claims) => {
                let exists = self
                    .file_exists(&file_name, &claims.sub)
                    .map_err(|_| Status::internal("Internal server error"))?;

                // If file does not exists in cache, it means user has not uploaded such file.
                if exists.is_none() {
                    return Err(Status::internal(format!("File {} not found", file_name)));
                }

                let files_and_download_path = self
                    .download_file(&claims.sub, &file_name)
                    .await
                    .map_err(|_| {
                        error!(
                            "Error downloading file {} for client {}",
                            file_name, &claims.sub
                        );
                        Status::internal("Internal server error")
                    })?;

                let content = file_to_bytes(&files_and_download_path[0].1)
                    .map_err(|_| Status::internal("Internal server error"))?;

                let merkle_proof_nodes = self
                    .generate_merkle_proof(files_and_download_path, &content)
                    .map_err(|_| {
                        error!("Error generating merkle proof for bytes {:?}", content);
                        Status::internal("Internal server error")
                    })?;

                let merkle_proof = Some(MerkleProof {
                    nodes: merkle_proof_nodes,
                });

                let (tx, rx) = mpsc::channel(4);
                // Here, spawn a new task to handle file reading and streaming
                tokio::spawn(async move {
                    let chunk = FileDownloadResponse {
                        content,
                        merkle_proof,
                    };

                    let _ = tx.send(Ok(chunk)).await.map_err(|err| {
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
