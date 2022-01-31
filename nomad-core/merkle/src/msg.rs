use serde::{Deserialize, Serialize};
use ethers_core::types::H256;

use crate::merkle_tree::IncrementalMerkle;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Insert { element: H256 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Root {},
    Count {},
    Tree {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RootResponse {
    pub root: H256,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CountResponse {
    pub count: usize,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TreeResponse {
    pub tree: IncrementalMerkle,
}
