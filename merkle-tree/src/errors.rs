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

// let total_levels = ((leaf_nodes as f64).log2().ceil() + 1.0) as usize;
