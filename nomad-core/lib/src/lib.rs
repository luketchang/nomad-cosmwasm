use cosmwasm_std::Addr;
use ethers_core::types::{SignatureError, H256};
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

/// Destination and destination-specific nonce combined in single field (
/// (destination << 32) & nonce)
pub fn destination_and_nonce(destination: u32, nonce: u32) -> u64 {
    assert!(destination < u32::MAX);
    assert!(nonce < u32::MAX);
    ((destination as u64) << 32) | nonce as u64
}

/// Convert ethers H256 to string (to_string implementation interprets diff)
pub fn h256_to_string(h256: H256) -> String {
    let bytes = h256.to_fixed_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

/// Convert cosmwasm_std::Addr into H256 (fixed 32 byte array)
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
