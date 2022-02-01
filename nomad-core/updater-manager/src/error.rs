use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Only the home can call slash")]
    SlashNotHome,

    #[error("{0}")]
    OwnableError(#[from] ownable::ContractError),
}
