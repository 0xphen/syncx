#[derive(thiserror::Error, Debug)]
pub enum SynxServerError {
    #[error("Failed to register client: {0}")]
    InvalidServerSettings(String),

    #[error("Failed to register client: {0}")]
    DatabaseConnectionError(String),

    #[error("{0}")]
    DbOptionsConfigurationError(String),

    #[error("Failed to get client #{0}")]
    ClientDataAccessError(String),

    #[error("Failed to create JWT token")]
    JWTTokenCreationError,

    #[error("Failed to parse int")]
    ParseIntError,

    #[error("Failed to hash password")]
    PasswordHashError,

    #[error("Failed to convert object to MongoDB document")]
    ObjectToDocConversionError,

    #[error("Failed to create client in MongoDB")]
    MongoDbClientCreationError,

    #[error("Invalid JWT timestamp when creating claims")]
    ClaimsTimestampError,

    #[error("invalid jwt token")]
    InvalidJWTTokenError,
}
