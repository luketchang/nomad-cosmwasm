use crate::nomad_base;
use ethers_core::types::H256;
use serde::{Deserialize, Serialize};

use crate::contract_msg::replica;

use crate::MessageStatus;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    ReplicaExecuteMsg(replica::ExecuteMsg),
    SetProven { leaf: H256 },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    ReplicaQueryMsg(replica::QueryMsg),
}
