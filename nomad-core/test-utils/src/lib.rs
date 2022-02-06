use cosmwasm_std::{Event, Response};
use ethers_core::types::H256;

mod updater_utils;
pub use updater_utils::*;

mod merkle_utils;
pub use merkle_utils::*;

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
