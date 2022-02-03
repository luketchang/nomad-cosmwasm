use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    OwnableError(#[from] ownable::ContractError),

    #[error("{0}")]
    NomadBaseError(#[from] nomad_base::ContractError),
}
