use cosmwasm_std::Event;

mod updater;
pub use updater::*;

pub fn event_attr_value_by_key(event: &Event, attribute_key: &str) -> Option<String> {
    event
        .attributes
        .iter()
        .find(|attr| attr.key == attribute_key)
        .map(|attr| attr.value.to_owned())
}
