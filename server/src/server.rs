extern crate proto;

use proto::syncx::{syncx_server::Syncx, CreateClientRequest, CreateClientResponse};
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::{auth, config::Config, definitions::ClientObject, definitions::Store};

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
}
