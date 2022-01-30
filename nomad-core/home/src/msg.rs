use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Dispatch { 
        destination_domain: u32,
        recipient_address: String,
        message: Vec<u8>,
    },
    DoubleUpdate {
        old_root: [u8; 32],
        new_roots: [[u8; 32]; 2],
        signature: Vec<u8>,
        signature_2: Vec<u8>,
    },
    ImproperUpdate {
        old_root: [u8; 32],
        new_root: [u8; 32],
        signature: Vec<u8>
    },
    RenounceOwnership {},
    TransferOwnership { new_owner: String },
    SetUpdater { updater: String },
    SetUpdaterManager { updater_manager: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    CommittedRoot {},
    Count {},
    HomeDomainHash {},
    LocalDomain {},
    Nonces { domain: u32 },
    Owner {},
    QueueContains { item: [u8; 32] },
    QueueEnd {},
    QueueLength {},
    Root {},
    State {},
    SuggestUpdate {},
    Tree {},
    Updater {},
    UpdaterManager {},
}
