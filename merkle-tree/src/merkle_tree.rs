use super::{errors::MerkleTreeError, utils::*};

use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct MerkleTree {
    pub nodes: Vec<Vec<String>>,
    indexes: HashMap<String, (usize, usize)>,
}

impl MerkleTree {
    pub fn new(leaf_bytes: &Vec<Vec<u8>>) -> Self {
        let leaves = Self::build_leaf_nodes(leaf_bytes);
        let mut indexes: HashMap<_, _> = HashMap::new();

        leaves.iter().enumerate().for_each(|(index, leaf)| {
            indexes.insert(leaf.clone(), (0, index));
        });

        let mut nodes = Vec::new();
        Self::from_leaves(leaves, &mut nodes, &mut indexes, 1);

        Self { nodes, indexes }
    }

    fn from_leaves(
        leaves: Vec<String>,
        nodes: &mut Vec<Vec<String>>,
        indexes: &mut HashMap<String, (usize, usize)>,
        level: usize,
    ) {
        let size_of_leaves = leaves.len();
        nodes.push(leaves.clone());

        if size_of_leaves <= 1 {
            return;
        }

        let mut new_leaves = Vec::new();
        let mut pos = 0;
        leaves.chunks(2).for_each(|chunk| {
            let left = chunk[0].clone();
            let right = chunk.get(1).unwrap_or(&left).clone();
            let leaf = hash_bytes(format!("{}{}", left, right).as_bytes());

            indexes.insert(leaf.clone(), (level, pos));
            new_leaves.push(leaf);
            pos += 1;
        });

        let level = level + 1;
        Self::from_leaves(new_leaves, nodes, indexes, level)
    }

    pub fn build_leaf_nodes(bytes: &Vec<Vec<u8>>) -> Vec<String> {
        let mut leaves = bytes
            .par_iter()
            .map(|block| hash_bytes(block))
            .collect::<Vec<String>>();

        leaves.sort();
        leaves
    }

    pub fn generate_merkle_proof(&self, leaf: &str) -> Result<Vec<(String, u8)>, MerkleTreeError> {
        if let Some((_level, leaf_index)) = self.indexes.get(leaf) {
            self.proof(*leaf_index)
        } else {
            Err(MerkleTreeError::InvalidNode)
        }
    }

    fn proof(&self, leaf_index: usize) -> Result<Vec<(String, u8)>, MerkleTreeError> {
        if leaf_index >= self.nodes[0].len() {
            return Err(MerkleTreeError::OutOfBounds); // Leaf index is out of bounds
        }

        let mut proof = Vec::new();
        let mut index = leaf_index;

        // Iterate through each level of the Merkle tree
        for level in self.nodes.iter().rev().skip(1).rev() {
            if level.len() <= index {
                return Err(MerkleTreeError::OutOfBounds);
            }

            // Find the sibling index (left or right)
            let sibling_index = if index % 2 == 0 {
                (index + 1).min(level.len() - 1)
            } else {
                index - 1
            };

            let is_left_sibling = (index % 2 == 0) as u8; // 1 for right sibling, 0 for left
            proof.push((level[sibling_index].clone(), is_left_sibling));
            // Move up to the parent level
            index /= 2;
        }

        Ok(proof)
    }

    pub fn verify(leaf: &str, merkle_proof: Vec<(String, u8)>, root_leaf: &str) -> (bool, String) {
        let mut current_leaf = leaf.to_string();

        for (sibling_hash, is_left_sibling) in merkle_proof {
            if is_left_sibling == 0 {
                current_leaf = hash_bytes(format!("{}{}", sibling_hash, current_leaf).as_bytes());
            } else {
                current_leaf = hash_bytes(format!("{}{}", current_leaf, sibling_hash).as_bytes());
            }
        }

        (current_leaf == root_leaf, current_leaf)
    }

    pub fn serialize(&self) -> Result<String, MerkleTreeError> {
        Ok(serde_json::to_string(&self).map_err(|_| MerkleTreeError::SerializeTreeError)?)
    }

    pub fn deserialize(&self, merkle_tree_str: &str) -> Result<Self, MerkleTreeError> {
        let deserialized: MerkleTree = serde_json::from_str(&merkle_tree_str)
            .map_err(|_| MerkleTreeError::DeserializeTreeError)?;
        Ok(deserialized)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, MerkleTreeError> {
        let merkle_tree_str =
            String::from_utf8(bytes.to_vec()).map_err(|_| MerkleTreeError::DeserializeTreeError)?;

        let deserialized: MerkleTree = serde_json::from_str(&merkle_tree_str)
            .map_err(|_| MerkleTreeError::DeserializeTreeError)?;

        Ok(deserialized)
    }

    pub fn leaf_nodes(&self) -> &Vec<String> {
        &self.nodes[0]
    }

    pub fn root(&self) -> &str {
        &self.nodes[self.nodes.len() - 1][0]
    }
}
