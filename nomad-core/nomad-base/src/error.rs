use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},
}

impl From<ownable::ContractError> for ContractError {
    fn from(err: ownable::ContractError) -> Self {
        match err {
            ownable::ContractError::Std(error) => ContractError::Std(error),
            ownable::ContractError::Unauthorized {} => ContractError::Unauthorized {},
        }
    }
}
