use ethers_core::types::H256;
use serde::{Deserialize, Serialize};

use crate::state::MessageStatus;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InstantiateMsg {
    pub local_domain: u32,
    pub remote_domain: u32,
    pub updater: String,
    pub committed_root: H256,
    pub optimistic_seconds: u64,
}

impl From<InstantiateMsg> for nomad_base::msg::InstantiateMsg {
    fn from(msg: InstantiateMsg) -> Self {
        Self {
            local_domain: msg.local_domain,
            updater: msg.updater,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Update {
        committed_root: H256,
        new_root: H256,
        signature: Vec<u8>,
    },
    DoubleUpdate {
        old_root: H256,
        new_roots: [H256; 2],
        signature: Vec<u8>,
        signature_2: Vec<u8>,
    },
    Prove {
        leaf: H256,
        proof: [H256; 32],
        index: u64,
    },
    Process {
        message: Vec<u8>,
    },
    ProveAndProcess {
        message: Vec<u8>,
        proof: [H256; 32],
        index: u64,
    },
    SetConfirmation {
        root: H256,
        confirm_at: u64,
    },
    SetOptimisticTimeout {
        optimistic_seconds: u64,
    },
    SetUpdater {
        updater: String,
    },
    RenounceOwnership {},
    TransferOwnership {
        new_owner: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    AcceptableRoot { root: H256 },
    ConfirmAt { root: H256 },
    MessageStatus { leaf: H256 },
    OptimisticSeconds {},
    RemoteDomain {},
    CommittedRoot {},
    HomeDomainHash {},
    LocalDomain {},
    State {},
    Updater {},
    Owner {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AcceptableRootResponse {
    pub acceptable: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ConfirmAtResponse {
    pub confirm_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct MessageStatusResponse {
    pub status: MessageStatus,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct OptimisticSecondsResponse {
    pub optimistic_seconds: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RemoteDomainResponse {
    pub remote_domain: u32,
}
