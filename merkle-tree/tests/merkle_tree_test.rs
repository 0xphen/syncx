mod common;

use common::*;
use merkle_tree::merkle_tree::MerkleTree;
use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    fn odd_leaves() -> Vec<Vec<u8>> {
        let mut leaf_bytes = BYTE_ARRAY_MATRIX.to_vec();
        leaf_bytes.pop();
        leaf_bytes
    }

    #[test]
    fn test_build_leaf_nodes() {
        let leaf_nodes = MerkleTree::build_leaf_nodes(&BYTE_ARRAY_MATRIX);
        assert!(vec![LA, LB, LC, LD] == leaf_nodes);
    }

    #[test]
    fn test_even_leaf_nodes_merkle_tree() {
        let merkle_tree = MerkleTree::new(&BYTE_ARRAY_MATRIX);

        let expected_merkle_tree_nodes = vec![
            vec![LA, LB, LC, LD],
            vec![H_LA_LB, H_LC_LD],
            vec![H_LALB_LCLD],
        ];
        // let mut expected_leaf_indexes: HashMap<String, (usize, usize)> = HashMap::new();
        assert!(merkle_tree.nodes == expected_merkle_tree_nodes);
    }

    #[test]
    fn test_odd_leaf_nodes_merkle_tree() {
        let merkle_tree = MerkleTree::new(&odd_leaves());

        let expected_merkle_tree_nodes =
            vec![vec![LA, LB, LD], vec![H_LA_LB, H_LD_LD], vec![H_LALB_LDLD]];

        assert!(merkle_tree.nodes == expected_merkle_tree_nodes);
    }

    #[test]
    fn test_valid_proof_for_even_tree() {
        let merkle_tree = MerkleTree::new(&BYTE_ARRAY_MATRIX);
        let proof = merkle_tree.generate_merkle_proof(LD).unwrap();
        assert!(proof == vec![LC, H_LA_LB]);
    }

    #[test]
    fn test_valid_proof_for_odd_tree() {
        let merkle_tree = MerkleTree::new(&odd_leaves());
        let proof = merkle_tree.generate_merkle_proof(LD).unwrap();
        assert!(proof == vec![LD, H_LA_LB]);
    }

    #[test]
    fn test_verify_merkle_proof() {
        let merkle_tree = MerkleTree::new(&BYTE_ARRAY_MATRIX);
        let leaf = LB;
        let proof = merkle_tree.generate_merkle_proof(leaf).unwrap();
        let valid_leaf = merkle_tree.verify(
            leaf,
            proof,
            &merkle_tree.nodes[merkle_tree.nodes.len() - 1][0],
        );

        assert!(valid_leaf);
    }

    #[test]
    fn test_serialize_and_deserialize_tree() {
        let merkle_tree = MerkleTree::new(&BYTE_ARRAY_MATRIX);
        let serialized_tree = merkle_tree.serialize().unwrap();

        let deserialized_tree = merkle_tree.deserialize(&serialized_tree).unwrap();
        assert!(deserialized_tree == merkle_tree);
    }
}
