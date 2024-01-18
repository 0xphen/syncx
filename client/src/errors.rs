use thiserror::Error;

#[derive(Error, Debug)]
pub enum SynxClientError {
    #[error("Failed to decode hex `0`")]
    FailedToRegisterClient(String),
}
