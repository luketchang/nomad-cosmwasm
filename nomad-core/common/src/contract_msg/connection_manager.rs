use serde::{Deserialize, Serialize};
use ethers_core::types::H256;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct InstantiateMsg {}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    UnenrollReplica {
        domain: u32,
        updater: H256,
        signature: Vec<u8>,
    },
    OwnerEnrollReplica { domain: u32, replica: String },
    OwnerUnenrollReplica { replica: String },
    SetWatcherPermission { domain: u32, watcher: H256, access: bool },
    SetHome { home: String },
    RenounceOwnership {},
    TransferOwnership { new_owner: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    LocalDomain {},
    DomainToReplica { domain: u32 },
    ReplicaToDomain { replica: String },
    WatcherPermission { domain: u32, watcher: String },
    IsReplica { replica: String },
    Owner {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct LocalDomainResponse {
    local_domain: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct WatcherPermissionResponse {
    has_permission: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct IsReplica {
    is_replica: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DomainToReplicaResponse {
    replica: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ReplicaToDomainResponse {
    domain: u32,
}


