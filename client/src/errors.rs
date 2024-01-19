#[derive(thiserror::Error, Debug)]
pub enum SynxClientError {
    #[error("Failed to register client: {0}")]
    ClientRegistrationError(String),

    #[error("Failed to determine home directory")]
    HomeDirDeterminationError,

    #[error("Failed to create config directory")]
    ConfigDirectoryCreationError,

    #[error("Failed to write to config file")]
    ConfigFileWriteError,
}
