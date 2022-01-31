use serde::{Deserialize, Serialize};
use ethers_core::types::H256;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Enqueue { item: H256 },
    Dequeue {},
    EnqueueBatch { items: Vec<H256> },
    DequeueBatch { number: u128 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Contains { item: H256 },
    LastItem {},
    Peek {},
    IsEmpty {},
    Length {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ContainsResponse {
    pub contains: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LastItemResponse {
    pub item: H256,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct PeekResponse {
    pub item: H256,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct IsEmptyResponse {
    pub is_empty: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LengthResponse {
    pub length: usize,
}
