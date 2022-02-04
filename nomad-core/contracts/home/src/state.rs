use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

pub const UPDATER_MANAGER: Item<Addr> = Item::new("updater_manager");
pub const NONCES: Map<u32, u32> = Map::new("nonces");
