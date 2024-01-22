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

    pub fn generate_merkle_proof(&self, leaf: &str) -> Result<Vec<String>, MerkleTreeError> {
        if let Some((_level, leaf_index)) = self.indexes.get(leaf) {
            self.proof(*leaf_index)
        } else {
            Err(MerkleTreeError::InvalidNode)
        }
    }

    fn proof(&self, leaf_index: usize) -> Result<Vec<String>, MerkleTreeError> {
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

            // Add the sibling's hash to the proof
            proof.push(level[sibling_index].clone());

            // Move up to the parent level
            index /= 2;
        }

        Ok(proof)
    }

    pub fn verify(&self, leaf: &str, merkle_proof: Vec<String>, root_leaf: &str) -> bool {
        let mut current_leaf = leaf.to_string();

        for hash in merkle_proof {
            let (a, b) = self.cmp_leaves(&current_leaf, hash.as_str());
            current_leaf = hash_bytes(format!("{}{}", a, b).as_bytes());
        }

        current_leaf == root_leaf
    }

    fn cmp_leaves<'a>(&self, a: &'a str, b: &'a str) -> (&'a str, &'a str) {
        let mut indexes = vec![
            ((self.indexes.get(a).unwrap()).1, a),
            ((self.indexes.get(b).unwrap()).1, b),
        ];

        indexes.sort_by(|&b, &a| b.0.cmp(&a.0));
        (indexes[0].1, indexes[1].1)
    }

    pub fn serialize(&self) -> Result<String, MerkleTreeError> {
        Ok(serde_json::to_string(&self).map_err(|_| MerkleTreeError::SerializeTreeError)?)
    }

    pub fn deserialize(&self, merkle_tree_str: &str) -> Result<Self, MerkleTreeError> {
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
