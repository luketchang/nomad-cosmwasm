use lib::Bytes32;
use schemars::JsonSchema;
use serde::{Serialize, Deserialize};

use ethers_core::types::H256;

use super::{
    hash_concat,
    TREE_DEPTH, ZERO_HASHES, Proof, merkle_root_from_branch,
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
/// An incremental merkle tree, modeled on the eth2 deposit contract
pub struct IncrementalMerkle {
    branch: [Bytes32; TREE_DEPTH],
    count: usize,
}

impl Default for IncrementalMerkle {
    fn default() -> Self {
        let mut branch: [Bytes32; TREE_DEPTH] = Default::default();
        branch
            .iter_mut()
            .enumerate()
            .for_each(|(i, elem)| *elem = ZERO_HASHES[i].into());
        Self { branch, count: 0 }
    }
}

impl IncrementalMerkle {
    /// Ingest a leaf into the tree.
    pub fn ingest(&mut self, element: H256) {
        let mut node = element;
        assert!(self.count < u32::MAX as usize);
        self.count += 1;
        let mut size = self.count;
        for i in 0..TREE_DEPTH {
            if (size & 1) == 1 {
                self.branch[i] = node.into();
                return;
            }
            node = hash_concat(self.branch[i], node);
            size /= 2;
        }
    }

    /// Calculate the current tree root
    pub fn root(&self) -> H256 {
        let mut node: H256 = Default::default();
        let mut size = self.count;

        self.branch.iter().enumerate().for_each(|(i, elem)| {
            node = if (size & 1) == 1 {
                hash_concat(elem, node)
            } else {
                hash_concat(node, ZERO_HASHES[i])
            };
            size /= 2;
        });

        node
    }

    /// Get the number of items in the tree
    pub fn count(&self) -> usize {
        self.count
    }

    /// Get the leading-edge branch.
    pub fn branch(&self) -> &[Bytes32; TREE_DEPTH] {
        &self.branch
    }

    /// Calculate the root of a branch for incremental given the index
    pub fn branch_root(item: H256, branch: [H256; TREE_DEPTH], index: usize) -> H256 {
        merkle_root_from_branch(item, &branch, 32, index)
    }

    /// Verify a incremental merkle proof of inclusion
    pub fn verify(&self, proof: &Proof) -> bool {
        let computed = IncrementalMerkle::branch_root(proof.leaf, proof.path, proof.index as usize);
        computed == self.root()
    }
}
