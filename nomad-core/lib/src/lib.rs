use cosmwasm_std::Addr;
use ethers_core::types::{H256, SignatureError};
use std::io::Write;

mod message;
pub use message::*;

mod traits;
pub use traits::*;

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

pub fn addr_to_bytes32(address: Addr) -> H256 {
    let addr = address.as_bytes().to_owned();
    let length = addr.len();
    if length > 32 {
        panic!("Address cannot be greater than 32 bytes long!")
    }

    let mut buf = vec![];
    let zeros = vec![0; 32 - length];
    buf.write(&zeros).unwrap();
    buf.write(&addr).unwrap();

    assert!(buf.len() == 32);

    let sized: [u8; 32] = buf.try_into().unwrap();
    H256::from(sized)
}
