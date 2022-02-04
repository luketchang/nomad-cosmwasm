use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const UPDATER: Item<Addr> = Item::new("updater_manager_updater");
pub const HOME: Item<Addr> = Item::new("updater_manager_home");
