use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Replica for domain {domain} does not exist")]
    ReplicaNotExists { domain: u32 },

    #[error("Not current updater: {address}")]
    NotCurrentUpdater { address: String },

    #[error("{0}")]
    SignatureError(#[from] ethers_core::types::SignatureError),

    #[error("{0}")]
    OwnableError(#[from] ownable::ContractError),
}
