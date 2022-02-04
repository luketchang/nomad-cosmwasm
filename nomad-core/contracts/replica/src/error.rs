use cosmwasm_std::StdError;
use ethers_core::types::H256;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Message with leaf {leaf} already proven")]
    MessageAlreadyProven { leaf: H256 },

    #[error("Message with leaf {leaf} not yet proven")]
    MessageNotYetProven { leaf: H256 },

    #[error("Not a current committed root: {old_root}")]
    NotCurrentCommittedRoot { old_root: H256 },

    #[error("Incorrectly tried processing message from domain {destination}")]
    WrongDestination { destination: u32 },

    #[error("Not updater signature")]
    NotUpdaterSignature,

    #[error("Failed to have recipient call process message. Leaf: {leaf}. Index: {index}")]
    FailedProveCall { leaf: H256, index: u64 },

    #[error("Failed to have recipient call process: {0}")]
    FailedProcessCall(String),

    #[error("Unknown reply message id received: {id}")]
    UnknownReplyMessage { id: u64 },

    #[error("{0}")]
    OwnableError(#[from] ownable::ContractError),

    #[error("{0}")]
    NomadBaseError(#[from] nomad_base::ContractError),
}