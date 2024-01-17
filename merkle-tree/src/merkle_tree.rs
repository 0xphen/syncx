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
        let leaves = Self::build_leaf_hashes(leaf_bytes);
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

        Self::from_leaves(new_leaves, nodes)
    }

    pub fn build_leaf_hashes(bytes: &Vec<Vec<u8>>) -> Vec<String> {
        bytes.par_iter().map(|block| hash_bytes(block)).collect()
    }
}
