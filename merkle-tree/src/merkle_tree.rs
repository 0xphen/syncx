use super::{errors::SynxError, utils::*};
use rayon::prelude::*;
use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub struct MerkleTree {
    pub nodes: Vec<Vec<String>>,
    leaf_indexes: HashMap<String, (usize, usize)>,
}

impl MerkleTree {
    pub fn new(leaf_bytes: &Vec<Vec<u8>>) -> Self {
        let leaves = Self::build_leaf_nodes(leaf_bytes);
        let mut leaf_indexes: HashMap<_, _> = HashMap::new();

        leaves.iter().enumerate().for_each(|(index, leaf)| {
            leaf_indexes.insert(leaf.clone(), (0, index));
        });

        let mut nodes = Vec::new();
        Self::from_leaves(leaves, &mut nodes);

        Self {
            nodes,
            leaf_indexes,
        }
    }

    fn from_leaves(leaves: Vec<String>, nodes: &mut Vec<Vec<String>>) {
        let size_of_leaves = leaves.len();
        nodes.push(leaves.clone());

        if size_of_leaves <= 1 {
            return;
        }

        let mut new_leaves = Vec::new();
        leaves.chunks(2).for_each(|chunk| {
            let left = chunk[0].clone();
            let right = chunk.get(1).unwrap_or(&left).clone();
            let leaf = hash_bytes(format!("{}{}", left, right).as_bytes());
            new_leaves.push(leaf);
        });

        println!("new_leaves{:?}  ", new_leaves);

        Self::from_leaves(new_leaves, nodes)
    }

    pub fn build_leaf_nodes(bytes: &Vec<Vec<u8>>) -> Vec<String> {
        let mut leaves = bytes
            .par_iter()
            .map(|block| hash_bytes(block))
            .collect::<Vec<String>>();

        leaves.sort();
        leaves
    }

    pub fn generate_merkle_proof(&self, leaf: &str) -> Result<Vec<String>, SynxError> {
        if let Some((_level, leaf_index)) = self.leaf_indexes.get(leaf) {
            self.proof(*leaf_index)
        } else {
            Err(SynxError::InvalidNode)
        }
    }

    fn proof(&self, leaf_index: usize) -> Result<Vec<String>, SynxError> {
        if leaf_index >= self.nodes[0].len() {
            return Err(SynxError::OutOfBounds); // Leaf index is out of bounds
        }

        let mut proof = Vec::new();
        let mut index = leaf_index;

        // Iterate through each level of the Merkle tree
        for level in self.nodes.iter().rev().skip(1).rev() {
            if level.len() <= index {
                return Err(SynxError::OutOfBounds);
            }

            // Find the sibling index (left or right)
            let sibling_index = if index % 2 == 0 {
                index + 1.min(level.len() - 1)
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
}
