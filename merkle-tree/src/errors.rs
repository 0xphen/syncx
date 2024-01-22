use thiserror::Error;

#[derive(Error, Debug)]
pub enum MerkleTreeError {
    #[error("Failed to decode hex")]
    FailedToDecodeHex,

    #[error("Invalid node")]
    InvalidNode,

    #[error("Index out of bounds")]
    OutOfBounds,

    #[error("Failed to serialize merkle tree")]
    SerializeTreeError,

    #[error("Failed to deserialize merkle tree")]
    DeserializeTreeError,
}
