use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Only the home can call slash")]
    SlashNotHome,

    #[error("Unknown reply message id received: {id}")]
    UnknownReplyMessage { id: u64 },

    #[error("Failed to call set updater on updater manager: {0}")]
    FailedSetUpdaterCall(String),

    #[error("{0}")]
    OwnableError(#[from] ownable::ContractError),
}
