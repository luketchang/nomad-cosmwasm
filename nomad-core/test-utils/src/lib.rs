use cosmwasm_std::{Event, Response};
use ethers_core::types::H256;

mod updater;
pub use updater::*;

/// Convert ethers H256 to string (to_string implementation interprets diff)
pub fn h256_to_string(h256: H256) -> String {
    let bytes = h256.to_fixed_bytes();
    String::from_utf8(bytes.to_vec()).unwrap()
}

/// Extract an event attribute's value from key
pub fn event_by_ty(res: &Response, ty: &str) -> Option<Event> {
    res.events
        .iter()
        .find(|event| event.ty == ty)
        .map(|event| event.to_owned())
}

/// Extract an event attribute's value from key
pub fn event_attr_value_by_key(event: &Event, attribute_key: &str) -> Option<String> {
    event
        .attributes
        .iter()
        .find(|attr| attr.key == attribute_key)
        .map(|attr| attr.value.to_owned())
}
