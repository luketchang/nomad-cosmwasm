use cosmwasm_std::{Addr, CanonicalAddr, Deps};
use ethers_core::types::H256;
use std::io::Write;

/// Destination and destination-specific nonce combined in single field (
/// (destination << 32) & nonce)
pub fn destination_and_nonce(destination: u32, nonce: u32) -> u64 {
    assert!(destination < u32::MAX);
    assert!(nonce < u32::MAX);
    ((destination as u64) << 32) | nonce as u64
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

pub fn bytes32_to_addr(deps: Deps, bytes32: H256) -> Addr {
    let canonical: CanonicalAddr = bytes32.as_bytes().into();
    deps.api.addr_humanize(&canonical).unwrap()
}