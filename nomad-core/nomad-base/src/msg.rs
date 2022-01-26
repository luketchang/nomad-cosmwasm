use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub local_domain: u32,
    pub updater: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Double update message
    DoubleUpdate {
        old_root: [u8; 32],
        new_root: [u8; 32],
        signature: String,
        signature_2: String,
    },
    /// Transfer ownership to address `0x0` (inherited from ownable)
    RenounceOwnership {},
    /// Transfer ownership to `newOwner` (inherited from ownable)
    TransferOwnership { new_owner: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Return current committed root
    CommittedRoot {},
    /// Hash of home domain concatenated with "NOMAD"
    HomeDomainHash {},
    /// Return contract's local domain
    LocalDomain {},
    /// Return contract's current state
    State {},
    /// Return updater address
    Updater {},
    /// Owner of contract (inherited from ownable)
    Owner {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CommittedRootResponse {
    /// Committed root
    pub committed_root: [u8; 32],
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct HomeDomainHashResponse {
    /// Home domain hash
    pub home_domain_hash: [u8; 32],
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LocalDomainResponse {
    /// Local domain
    pub local_domain: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct StateResponse {
    /// State
    pub state: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct UpdaterResponse {
    /// Updater address
    pub updater: String,
}
