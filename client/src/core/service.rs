pub mod client {
    extern crate proto;

    use crate::core::{context::Context, utils::*};
    use proto::syncx::{syncx_client::SyncxClient, CreateClientRequest, CreateClientResponse};
    use std::path::{Path, PathBuf};

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

    pub fn list_files(path: &str) {
        let files = list_files_in_dir(&PathBuf::from(path));
        println!("FILES: {:?}", files.unwrap());
    }
}
