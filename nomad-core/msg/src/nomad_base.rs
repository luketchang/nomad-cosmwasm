use crate::ownable;
use ethers_core::types::H256;
use serde::{Deserialize, Serialize};

use lib::States;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InstantiateMsg {
    pub local_domain: u32,
    pub updater: String,
}

impl From<InstantiateMsg> for ownable::InstantiateMsg {
    fn from(_: InstantiateMsg) -> Self {
        ownable::InstantiateMsg {}
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    /// Transfer ownership to address `0x0` (inherited from ownable)
    RenounceOwnership {},
    /// Transfer ownership to `newOwner` (inherited from ownable)
    TransferOwnership { new_owner: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct CommittedRootResponse {
    /// Committed root
    pub committed_root: H256,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct HomeDomainHashResponse {
    /// Home domain hash
    pub home_domain_hash: H256,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LocalDomainResponse {
    /// Local domain
    pub local_domain: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct StateResponse {
    /// State
    pub state: States,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct UpdaterResponse {
    /// Updater address
    pub updater: String,
}
