use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub nonces: HashMap<u32, u32>,
    pub updater_manager: Addr,
}

pub const UPDATER_MANAGER: Item<Addr> = Item::new("updater_manager");
pub const NONCES: Map<u32, u32> = Map::new("nonces");
