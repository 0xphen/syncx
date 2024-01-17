use super::{errors::SynxError, utils::*};
use rayon::prelude::*;
use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub struct MerkleTree {
    pub nodes: Vec<String>,
    leaf_indexes: HashMap<String, usize>,
}

impl MerkleTree {
    pub fn new(leaf_bytes: &Vec<Vec<u8>>) -> Self {
        let leaves = Self::build_leaf_hashes(leaf_bytes);
        Self::from_leaves(leaves)
    }

    pub fn from_leaves(leaves: Vec<String>) -> MerkleTree {
        let total_nodes = Self::tree_size(leaves.len());
        let mut nodes = vec![String::new(); total_nodes];
        let mut leaf_indexes = HashMap::new();

        let mut index = total_nodes - leaves.len();
        for hash in leaves.iter() {
            leaf_indexes.insert(hash.clone(), index);
            nodes[index] = hash.clone();
            index += 1;
        }

        index = total_nodes - leaves.len();
        for ptr in (0..index).rev() {
            let left = nodes[2 * ptr + 1].clone();
            let right = nodes.get(ptr * 2 + 2).unwrap_or(&left).clone();
            let parent_hash = hash_bytes(format!("{}{}", left, right).as_bytes());
            nodes[ptr] = parent_hash;
        }

        MerkleTree {
            nodes,
            leaf_indexes,
        }
    }

    pub fn generate_merkle_proof(&self, leaf: &str) -> Result<Vec<String>, SynxError> {
        if let Some(leaf_index) = self.leaf_indexes.get(leaf) {
            self.proof(*leaf_index)
        } else {
            Err(SynxError::InvalidNode)
        }
    }

    fn proof(&self, leaf_index: usize) -> Result<Vec<String>, SynxError> {
        if leaf_index >= self.nodes.len() {
            return Err(SynxError::OutOfBounds);
        }

        let mut proof = Vec::new();
        let mut current_index = leaf_index;

        while current_index > 0 {
            let sibling_index = if current_index % 2 == 0 {
                current_index - 1 // Left sibling
            } else {
                current_index + 1
            };

            proof.push(self.nodes[sibling_index].clone());
            current_index = (current_index - 1) / 2;
        }

        Ok(proof)
    }

    pub fn build_leaf_hashes(bytes: &Vec<Vec<u8>>) -> Vec<String> {
        bytes.par_iter().map(|block| hash_bytes(block)).collect()
    }

    fn tree_size(leaf_size: usize) -> usize {
        let mut total_nodes = leaf_size;
        let mut next_level_size = leaf_size as f64;

        while next_level_size > 1_f64 {
            next_level_size = (next_level_size / 2_f64).ceil();
            total_nodes += next_level_size as usize;
        }

        total_nodes
    }
}
