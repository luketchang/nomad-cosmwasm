use ethers_core::types::H160;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InstantiateMsg {
    pub updater: H160,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    SetHome { home: String },
    SetUpdater { updater: H160 },
    SlashUpdater { reporter: String },
    RenounceOwnership {},
    TransferOwnership { new_owner: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Updater {},
    Owner {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct UpdaterResponse {
    pub updater: H160,
}
