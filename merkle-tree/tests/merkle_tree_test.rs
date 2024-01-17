mod common;

use common::*;
use merkle_tree::merkle_tree::MerkleTree;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_build_leaf_nodes() {
        let leaf_nodes = MerkleTree::build_leaf_hashes(&BYTE_ARRAY_MATRIX);
        assert!(vec![NODE_7, NODE_6, NODE_5, NODE_4] == leaf_nodes);
    }

    #[test]
    fn test_even_leaf_nodes_merkle_tree() {
        let merkle_tree = MerkleTree::new(&BYTE_ARRAY_MATRIX);

        let expected_merkle_tree_nodes = vec![
            vec![NODE_7, NODE_6, NODE_5, NODE_4],
            vec![NODE_2, NODE_3],
            vec![NODE_1],
        ];

        assert!(merkle_tree.nodes == expected_merkle_tree_nodes);
    }

    #[test]
    fn test_odd_leaf_nodes_merkle_tree() {
        let mut leaves = BYTE_ARRAY_MATRIX.to_vec();
        leaves.pop();
        let merkle_tree = MerkleTree::new(&leaves);

        let expected_merkle_tree_nodes = vec![
            vec![NODE_7, NODE_6, NODE_5],
            vec![NODE_2, NODE_8],
            vec![NODE_9],
        ];

        assert!(merkle_tree.nodes == expected_merkle_tree_nodes);
    }
}
