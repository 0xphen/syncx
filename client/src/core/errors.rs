#[derive(thiserror::Error, Debug)]
pub enum SynxClientError {
    #[error("Failed to create config directory")]
    ConfigDirectoryCreationError,

    #[error("Failed to write to config file")]
    ConfigFileWriteError,
}
