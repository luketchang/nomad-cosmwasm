use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Invalid double update submitted")]
    InvalidDoubleUpdate {},

    #[error("Function not callable in a failed state")]
    FailedState {},

    #[error("{0}")]
    SignatureError(#[from] ethers_core::types::SignatureError),

    #[error("{0}")]
    OwnableError(#[from] ownable::ContractError),
}
