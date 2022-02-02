use serde::{Deserialize, Serialize};
use ethers_core::types::H256;

use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum MessageStatus {
    None,
    Proven,
    Processed
}

pub const REMOTE_DOMAIN: Item<u32> = Item::new("remote_domain");
pub const OPTIMISTIC_SECONDS: Item<u128> = Item::new("optimistic_seconds");
pub const CONFIRM_AT: Map<H256, u128> = Map::new("confirm_at");
pub const MESSAGES: Map<H256, MessageStatus> = Map::new("messages");
