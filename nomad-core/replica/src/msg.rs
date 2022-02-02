use serde::{Deserialize, Serialize};
use ethers_core::types::H256;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InstantiateMsg {}

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
        index: u128,
     },
    Process { message: Vec<u8> },
    ProveAndProcess {
        leaf: H256,
        proof: [H256; 32],
        index: u128,
    },
    SetConfirmation {
        root: H256,
        confirm_at: u128,
    },
    SetOptimisticTimeout { optimistic_seconds: u128 },
    SetUpdater { updater: String },
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
    Owner {}
}
