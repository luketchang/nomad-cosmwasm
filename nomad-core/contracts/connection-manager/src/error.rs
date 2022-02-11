use cosmwasm_std::StdError;
use ethers_core::types::{H160, H256};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Replica for domain {domain} does not exist")]
    NotReplicaExists { domain: u32 },

    #[error("Not current updater: {address}")]
    NotCurrentUpdater { address: String },

    #[error(
        "Watcher {watcher} does not have permissions for replica {replica} on domain {domain}"
    )]
    NotWatcherPermission {
        watcher: H160,
        replica: H256,
        domain: u32,
    },

    #[error("{0}")]
    SignatureError(#[from] ethers_core::types::SignatureError),

    #[error("{0}")]
    OwnableError(#[from] ownable::ContractError),
}
