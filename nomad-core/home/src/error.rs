use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Message length {length} too long")]
    MsgTooLong { length: u128 },

    #[error("Not updater signature")]
    NotUpdaterSignature,

    #[error("Not improper update")]
    NotImproperUpdate,

    #[error("Not updater manager")]
    NotUpdaterManager { address: String },

    #[error("{0}")]
    OwnableError(#[from] ownable::ContractError),

    #[error("{0}")]
    MerkleError(#[from] merkle::ContractError),

    #[error("{0}")]
    QueueError(#[from] queue::ContractError),

    #[error("{0}")]
    NomadBaseError(#[from] nomad_base::ContractError),
}
