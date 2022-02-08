use common::MessageStatus;
use cw_storage_plus::{Item, Map};

pub const CHAIN_ADDR_LENGTH_BYTES: Item<usize> = Item::new("replica_chain_addr_length");

pub const REMOTE_DOMAIN: Item<u32> = Item::new("replica_remote_domain");
pub const OPTIMISTIC_SECONDS: Item<u64> = Item::new("replica_optimistic_seconds");

// Kludge: can't use H256 for primary key, can't use u256 for timestamps
pub const CONFIRM_AT: Map<&[u8], u64> = Map::new("replica_confirm_at");
pub const MESSAGES: Map<&[u8], MessageStatus> = Map::new("replica_messages");

pub const PROCESS_GAS: Item<u64> = Item::new("replica_process_gas");
pub const RESERVE_GAS: Item<u64> = Item::new("replica_reserve_gas");
