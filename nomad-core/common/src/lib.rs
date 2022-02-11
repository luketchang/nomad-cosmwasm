use ethers_core::types::SignatureError;
use serde::{Deserialize, Serialize};

mod message;
pub use message::*;

mod contract_msg;
pub use contract_msg::*;

pub mod merkle_tree;

mod utils;
pub use utils::*;

mod traits;
pub use traits::*;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum States {
    UnInitialized,
    Active,
    Failed,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum MessageStatus {
    None,
    Pending,
    Processed,
}

impl Default for MessageStatus {
    fn default() -> Self {
        Self::None
    }
}

/// Error types for Nomad
#[derive(Debug, thiserror::Error)]
pub enum NomadError {
    /// Signature Error pasthrough
    #[error(transparent)]
    SignatureError(#[from] SignatureError),

    /// IO error from Read/Write usage
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}
