use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

pub const CHAIN_ADDR_LENGTH_BYTES: Item<usize> = Item::new("connection_manager_chain_addr_length");

pub const HOME: Item<Addr> = Item::new("connection_manager_home");

pub const DOMAIN_TO_REPLICA: Map<u32, Addr> = Map::new("connection_manager_domain_to_replica");
pub const REPLICA_TO_DOMAIN: Map<Addr, u32> = Map::new("connection_manager_replica_to_domain");

// Hash of 20-byte watcher address + domain --> permission
pub const WATCHER_PERMISSIONS: Map<&[u8], bool> =
    Map::new("connection_manager_watcher_permissions");
