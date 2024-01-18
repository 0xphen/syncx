#[derive(thiserror::Error, Debug)]
pub enum SynxClientError {
    #[error("Failed to register client: {0}")]
    FailedToRegisterClient(String),
}
