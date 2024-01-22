pub mod client {
    extern crate common;

    use crate::core::{context::Context, utils::*};
    use common::syncx::{
        syncx_client::SyncxClient, CreateClientRequest, CreateClientResponse, FileDownloadRequest,
        FileUploadRequest,
    };
    use log::info;
    use merkle_tree::utils::hash_bytes;
    use std::fs;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use tokio::{fs::File, io::AsyncReadExt, sync::mpsc};
    use tokio_stream::wrappers::ReceiverStream;

    const DEFAULT_ZIP_FILE: &str = "uploads.zip";

    pub async fn register_client(
        syncx_client: &mut SyncxClient<tonic::transport::Channel>,
        password: String,
        context: &mut Context,
    ) {
        let response = syncx_client
            .register_client(CreateClientRequest {
                password: password.to_string(),
            })
            .await;

        match response {
            Ok(res) => {
                let CreateClientResponse { id, jwt_token } = res.into_inner();

                context.app_config.set_id(id);
                context.app_config.set_password(password);
                context.app_config.set_jwt(jwt_token);
                context
                    .app_config
                    .write(&context.path)
                    .unwrap_or_else(|e| panic!("Failed to update app state: {}", e));
            }
            Err(e) => {
                panic!("Failed to create user account {:?}", e);
            }
        }
    }

    pub async fn upload_files(
        syncx_client: &mut SyncxClient<tonic::transport::Channel>,
        path: &str,
        context: &mut Context,
    ) {
        let files = list_files_in_dir(&PathBuf::from(path)).unwrap();
        let merkle_tree = generate_merkle_tree(&files).unwrap();

        let zip_path = PathBuf::from(path).join(DEFAULT_ZIP_FILE);
        zip_files(&files, &zip_path);

        context
            .app_config
            .set_merkle_root(merkle_tree.root().to_string());

        context.app_config.write(&context.path);

        let file_contents = tokio::fs::read(&zip_path).await.unwrap();
        let checksum = hash_bytes(&file_contents);

        let requests = file_contents
            .chunks(4096)
            .map(|chunk| FileUploadRequest {
                jwt: context.app_config.jwt.to_string(),
                content: chunk.to_vec(),
            })
            .collect::<Vec<FileUploadRequest>>();

        let mut request = tonic::Request::new(tokio_stream::iter(requests));
        request
            .metadata_mut()
            .insert("checksum", checksum.parse().unwrap());

        match syncx_client.upload_files(request).await {
            Ok(response) => println!("SUMMARY: {:?}", response.into_inner()),
            Err(e) => println!("something went wrong: {:?}", e),
        }
    }

    pub async fn download_file(
        syncx_client: &mut SyncxClient<tonic::transport::Channel>,
        file_name: &str,
        output_path: &PathBuf,
        context: &mut Context,
    ) {
        println!("Downloading to: {:?}", output_path);
        let request = tonic::Request::new(FileDownloadRequest {
            jwt: context.app_config.jwt.to_string(),
            file_name: file_name.to_string(),
        });

        let mut stream = syncx_client
            .download_file(request)
            .await
            .unwrap()
            .into_inner();

        let mut file = fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&output_path)
            .unwrap();

        let mut merkle_proof: Option<Vec<String>> = None;
        while let Some(response) = stream.message().await.unwrap() {
            if merkle_proof.is_none() {
                merkle_proof = Some(response.merkle_proof);
            }

            let chunk = response.content;
            file.write_all(&chunk).unwrap();
        }

        println!("Download complete...");
    }
}
