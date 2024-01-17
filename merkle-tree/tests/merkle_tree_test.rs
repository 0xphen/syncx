mod common;

use common::*;
use merkle_tree::merkle_tree::MerkleTree;
use std::collections::HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_build_leaf_nodes() {
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
        let mut leaves = BYTE_ARRAY_MATRIX.to_vec();
        leaves.pop();
        let merkle_tree = MerkleTree::new(&leaves);

        let expected_merkle_tree_nodes =
            vec![vec![LA, LB, LD], vec![H_LA_LB, H_LD_LD], vec![H_LALB_LDLD]];

        assert!(merkle_tree.nodes == expected_merkle_tree_nodes);
    }

    #[test]
    fn should_generate_a_valid_proof_for_an_even_tree() {
        let merkle_tree = MerkleTree::new(&BYTE_ARRAY_MATRIX);
        let proof = merkle_tree.generate_merkle_proof(LD).unwrap();
        assert!(proof == vec![LC, H_LA_LB]);
    }
}
