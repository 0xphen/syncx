mod common;

use common::*;
use merkle_tree::merkle_tree::MerkleTree;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_build_leaf_nodes() {
        let leaf_nodes = MerkleTree::build_leaf_hashes(&BYTE_ARRAY_MATRIX);
        assert!(vec![NODE_1, NODE_2, NODE_3, NODE_4] == leaf_nodes);
    }

    #[test]
    fn test_even_leaf_nodes_merkle_tree() {
        let merkle_tree = MerkleTree::new(&BYTE_ARRAY_MATRIX);

        let expected_merkle_tree_nodes =
            vec![NODE_7, NODE_6, NODE_5, NODE_1, NODE_2, NODE_3, NODE_4];

        assert!(merkle_tree.nodes == expected_merkle_tree_nodes);
    }
}
