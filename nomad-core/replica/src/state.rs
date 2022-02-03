use serde::{Deserialize, Serialize};

use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum MessageStatus {
    None,
    Proven,
    Processed,
}

impl Default for MessageStatus {
    fn default() -> Self {
        Self::None
    }
}

pub const REMOTE_DOMAIN: Item<u32> = Item::new("remote_domain");
pub const OPTIMISTIC_SECONDS: Item<u64> = Item::new("optimistic_seconds");

// Kludge: can't use H256 for primary key, can't use u256 for timestamps
pub const CONFIRM_AT: Map<&[u8], u64> = Map::new("confirm_at");
pub const MESSAGES: Map<&[u8], MessageStatus> = Map::new("messages");

pub const PROCESS_GAS: Item<u64> = Item::new("process_gas");
pub const RESERVE_GAS: Item<u64> = Item::new("reserve_gas");
