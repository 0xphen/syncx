#[derive(thiserror::Error, Debug)]
pub enum SynxServerError {
    #[error("Invalid server settings: {0}")]
    InvalidServerConfigError(String),

    #[error("Failed to register client: {0}")]
    InvalidServerSettings(String),

    #[error("{0}")]
    DbOptionsConfigurationError(String),

    #[error("Failed to connect to database: {0}")]
    DatabaseConnectionError(String),

    #[error("Failed to connect to Redis: {0}")]
    RedisConnectionError(String),

    #[error("Failed to obtain a Redis connection: {0}")]
    FailedToObtainRedisConnection(String),

    #[error("Failed to get client #{0}")]
    ClientDataAccessError(String),

    #[error("Failed to retrieve Redis data: {0}")]
    RedisTypeError(String),

    #[error("Failed to execute Redis command: {0}")]
    RedisCMDError(String),

    #[error("Failed to get connection from redis pool: {0}")]
    RedisPoolError(String),

    #[error("Failed to create JWT token")]
    JWTTokenCreationError,

    #[error("Failed to convert string to struct: {0}")]
    DeserializationError(String),

    #[error("Failed to convert struct to string: {0}")]
    SerializationError(String),

    #[error("Failed to parse int")]
    ParseIntError,

    #[error("Failed to read response bytes")]
    HttpReadBytesError,

    #[error("Failed to hash password")]
    PasswordHashError,

    #[error("Failed to convert object to MongoDB document")]
    ObjectToDocConversionError,

    #[error("Failed to create client in MongoDB")]
    MongoDbClientCreationError,

    #[error("Invalid JWT timestamp when creating claims")]
    DownloadError,

    #[error("invalid jwt token")]
    InvalidJWTTokenError,

    #[error("Failed to queue job id #{0}")]
    DequeueJobError(String),

    #[error("Failed to read file")]
    ReadFileError,

    #[error("Failed to write to file")]
    WriteAllError,

    #[error("Failed to generate merkle tree")]
    MerkleTreeGenerationError,

    #[error("Failed to create directory")]
    CreateDirectoryError,

    #[error("Failed to create file")]
    CreateFileError,

    #[error("Failed to open file")]
    FileOpenError,

    #[error("Failed to unzip file")]
    UnzipError,

    #[error("Failed to list files in dir")]
    ListFilesError,

    #[error("File upload failed {0}")]
    UploadFileRequestError(String),

    #[error("Failed to serialize merkle tree")]
    SerializeTreeError,

    #[error("Failed to deserialize merkle tree")]
    DeserializeTreeError,
}
