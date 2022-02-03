use cosmwasm_std::StdError;
use ethers_core::types::H256;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Message length {length} too long")]
    MsgTooLong { length: u64 },

    #[error("Not a current committed root: {old_root}")]
    NotCurrentCommittedRoot { old_root: H256 },

    #[error("Not updater signature")]
    NotUpdaterSignature,

    #[error("Not improper update")]
    NotImproperUpdate,

    #[error("Not updater manager: {address}")]
    NotUpdaterManager { address: String },

    #[error("Unknown reply message id received: {id}")]
    UnknownReplyMessage { id: u64 },

    #[error("Failed reply to slash updater: {0}")]
    FailedSlashUpdaterReply(String),

    #[error("{0}")]
    OwnableError(#[from] ownable::ContractError),

    #[error("{0}")]
    MerkleError(#[from] merkle::ContractError),

    #[error("{0}")]
    QueueError(#[from] queue::ContractError),

    #[error("{0}")]
    NomadBaseError(#[from] nomad_base::ContractError),
}
