use cosmwasm_std::Addr;
use cw_storage_plus::Item;
use ethers_core::types::H160;

pub const UPDATER: Item<H160> = Item::new("updater_manager_updater");
pub const HOME: Item<Addr> = Item::new("updater_manager_home");
