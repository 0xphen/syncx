use thiserror::Error;

#[derive(Error, Debug)]
pub enum SynxError {
    #[error("Failed to decode hex")]
    FailedToDecodeHex,

    #[error("Invalid node")]
    InvalidNode,

    #[error("Index out of bounds")]
    OutOfBounds,
}
