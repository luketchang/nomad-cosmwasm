use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};
use ethers_core::types::{H160, H256};
use common::GovernanceBatchStatus;

// Immutable variables
pub const LOCAL_DOMAIN: Item<u32> = Item::new("governance_router_local_domain");
pub const RECOVERY_TIMELOCK: Item<u64> = Item::new("governance_router_recovery_timelock");

pub const RECOVERY_ACTIVE_AT: Item<u64> = Item::new("governance_router_recovery_active_at");
pub const RECOVERY_MANAGER: Item<H160> = Item::new("governance_router_recovery_manager");
pub const GOVERNOR: Item<H160> = Item::new("governance_router_governor");
pub const GOVERNOR_DOMAIN: Item<u32> = Item::new("recovery_manager_governor_domain");
pub const XAPP_CONNECTION_MANAGER: Item<Addr> = Item::new("governance_router_xapp_connection_manager");
pub const ROUTERS: Map<u32, H256> = Map::new("governance_router_routers");
pub const DOMAINS: Item<Vec<u32>> = Item::new("governance_router_domains");
pub const INBOUND_CALL_BATCHES: Map<&[u8], bool> = Map::new("governance_router_incoming_call_batches");