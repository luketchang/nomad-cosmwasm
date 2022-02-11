use super::{merkle, nomad_base, ownable, queue};
use ethers_core::types::{H160, H256};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InstantiateMsg {
    pub local_domain: u32,
    pub updater: H160,
}

impl From<InstantiateMsg> for ownable::InstantiateMsg {
    fn from(_: InstantiateMsg) -> Self {
        ownable::InstantiateMsg {}
    }
}

impl From<InstantiateMsg> for queue::InstantiateMsg {
    fn from(_: InstantiateMsg) -> Self {
        queue::InstantiateMsg {}
    }
}

impl From<InstantiateMsg> for merkle::InstantiateMsg {
    fn from(_: InstantiateMsg) -> Self {
        merkle::InstantiateMsg {}
    }
}

impl From<InstantiateMsg> for nomad_base::InstantiateMsg {
    fn from(msg: InstantiateMsg) -> Self {
        nomad_base::InstantiateMsg {
            local_domain: msg.local_domain,
            updater: msg.updater,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Dispatch {
        destination: u32,
        recipient: String,
        message_body: Vec<u8>,
    },
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
    ImproperUpdate {
        old_root: H256,
        new_root: H256,
        signature: Vec<u8>,
    },
    SetUpdater {
        updater: H160,
    },
    SetUpdaterManager {
        updater_manager: String,
    },
    RenounceOwnership {},
    TransferOwnership {
        new_owner: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    CommittedRoot {},
    Count {},
    HomeDomainHash {},
    LocalDomain {},
    Nonces { domain: u32 },
    Owner {},
    QueueContains { item: H256 },
    QueueEnd {},
    QueueLength {},
    Root {},
    State {},
    SuggestUpdate {},
    // Tree {},
    Updater {},
    UpdaterManager {},

    MaxMessageBodyBytes {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct NoncesResponse {
    pub next_nonce: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SuggestUpdateResponse {
    pub committed_root: H256,
    pub new_root: H256,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct UpdaterManagerResponse {
    pub updater_manager: String,
}
