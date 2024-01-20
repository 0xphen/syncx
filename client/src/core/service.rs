pub mod client {
    extern crate proto;

    use crate::core::{context::Context, utils::*};
    use proto::syncx::{
        syncx_client::SyncxClient, CreateClientRequest, CreateClientResponse, FileUploadRequest,
    };
    use std::path::{Path, PathBuf};
    use tokio::{fs::File, io::AsyncReadExt, sync::mpsc};
    use tokio_stream::wrappers::ReceiverStream;

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

        let zip_path = PathBuf::from(path).join("uploads.zip");
        zip_files(&files, &zip_path);

        context
            .app_config
            .set_merkle_root(merkle_tree.root().to_string());

        context.app_config.write(&context.path);
        let file_contents = tokio::fs::read(&zip_path).await.unwrap();

        let requests = file_contents
            .chunks(4096)
            .map(|chunk| FileUploadRequest {
                jwt: context.app_config.jwt.to_string(),
                content: chunk.to_vec(),
            })
            .collect::<Vec<FileUploadRequest>>();

        let request = tonic::Request::new(tokio_stream::iter(requests));

        match syncx_client.upload_files(request).await {
            Ok(response) => println!("SUMMARY: {:?}", response.into_inner()),
            Err(e) => println!("something went wrong: {:?}", e),
        }
    }

    async fn stream_file(
        syncx_client: &mut SyncxClient<tonic::transport::Channel>,
        jwt: &str,
        zip_path: PathBuf,
    ) {
        let mut file = File::open(zip_path).await.unwrap();
        let (sender, receiver) = mpsc::channel(2);
        let stream = ReceiverStream::new(receiver);
        let jwt = jwt.to_owned();

        tokio::spawn(async move {
            let (tx, _) = tokio::sync::mpsc::channel(2);
            let mut buf = vec![0; 1024];

            while let Ok(n) = file.read(&mut buf).await {
                if n == 0 {
                    break;
                }

                let content = buf[..n].to_vec();
                println!("Content: {:?}", content);

                let request = FileUploadRequest {
                    jwt: jwt.to_string(),
                    content,
                };

                println!("{:?}", tx.send(request).await);
            }
        });

        let response = syncx_client.upload_files(stream).await.unwrap();
        println!("Response: {:?}", response.into_inner());
    }
}
