use serde::{Deserialize, Serialize};

use super::{merkle_root_from_branch, TREE_DEPTH};

/// A merkle proof object. The leaf, its path to the root, and its index in the
/// tree.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
pub struct Proof {
    /// The leaf
    pub leaf: [u8; 32],
    /// The index
    pub index: usize,
    /// The merkle branch
    pub path: [[u8; 32]; TREE_DEPTH],
}

impl Proof {
    /// Calculate the merkle root produced by evaluating the proof
    pub fn root(&self) -> [u8; 32] {
        merkle_root_from_branch(self.leaf, self.path.as_ref(), TREE_DEPTH, self.index)
    }
}
