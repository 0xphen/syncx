use hex;
use sha2::{Digest, Sha256};

pub fn hash_bytes(byte: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(byte);
    hex::encode(hasher.finalize())
}
