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

    #[error("Failed to generate merkle root")]
    MerkleRootGenerationError,

    #[error("Failed to convert file to bytes:  {0}")]
    FileToBytesConversionError(String),
}
