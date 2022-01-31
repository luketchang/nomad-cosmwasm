use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Transfer ownership to address `0x0`
    RenounceOwnership {},
    /// Transfer ownership to `newOwner`
    TransferOwnership { new_owner: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Returns contract owner
    Owner {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct OwnerResponse {
    /// The owner
    pub owner: String,
}
