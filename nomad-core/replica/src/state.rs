use ethers_core::types::H256;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum MessageStatus {
    None,
    Proven,
    Processed,
}

pub const REMOTE_DOMAIN: Item<u32> = Item::new("remote_domain");
pub const OPTIMISTIC_SECONDS: Item<u64> = Item::new("optimistic_seconds");

// Kludge: can't use H256 for primary key, can't use u256 for timestamps
pub const CONFIRM_AT: Map<&[u8], u64> = Map::new("confirm_at");
pub const MESSAGES: Map<&[u8], MessageStatus> = Map::new("messages");
