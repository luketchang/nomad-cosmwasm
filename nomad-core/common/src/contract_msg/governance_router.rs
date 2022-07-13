use ethers_core::types::{H160, H256};
use serde::{Deserialize, Serialize};
use crate::{typed_msg::governance_message::Call, GovernanceBatchStatus};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InstantiateMsg {
    pub local_domain: u32,
    pub recovery_timelock: u64,
    pub xapp_connection_manager: H160,
    pub recovery_manager: H160,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    ExecuteCallBatch { calls: Vec<Call> },
    ExecuteGovernanceActions {
        local_calls: Vec<Call>,
        domains: Vec<u32>,
        remote_calls: Vec<Call>,
    },
    ExitRecovery {},
    InitiateRecoveryTimelock {},
    SetRouterGlobal { 
        domain: u32,
        router: H256,
    },
    SetRouterLocal {
        domain: u32,
        router: H256,
    },
    SetXAppConnectionManager {
        xapp_connection_manager: String,
    },
    TransferGovernor {
        new_domain: u32,
        new_governor: H160,
    },
    TransferRecoveryManager {
        new_recovery_manager: H160,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Domains {},
    Governor {},
    GovernorDomain {},
    InRecovery {},
    InboundCallBatches { batch_hash: H256 },
    LocalDomain {},
    RecoveryActiveAt {},
    RecoveryManager {},
    RecoveryTimelock {},
    Routers { domain: u32 },
    XAppConnectionManager {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DomainsResponse {
    pub domains: Vec<u32>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GovernorResponse {
    pub governor: H160,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GovernorDomainResponse {
    pub governor_domain: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InRecoveryResponse {
    pub in_recovery: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InboundCallBatchesResponse {
    pub status: GovernanceBatchStatus,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LocalDomainResponse {
    pub local_domain: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RecoveryActiveAtResponse {
    pub active_at: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RecoveryManagerResponse {
    pub recovery_manager: H160,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RecoveryTimelockResponse {
    pub recovery_timelock: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct RoutersResponse {
    pub router: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct XAppConnectionManagerResponse {
    pub xapp_connection_manager: String,
}