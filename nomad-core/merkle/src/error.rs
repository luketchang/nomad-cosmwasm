use cosmwasm_std::StdError;
use thiserror::Error;

use crate::merkle_tree;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),
}
