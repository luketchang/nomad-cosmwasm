use ethers_core::types::H256;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Addr;
use cw_storage_plus::Item;
use lib::States;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct State {
    pub local_domain: u32,
    pub updater: Addr,
    pub state: States,
    pub committed_root: H256,
}

pub const LOCAL_DOMAIN: Item<u32> = Item::new("nomad_base_local_domain");
pub const UPDATER: Item<Addr> = Item::new("nomad_base_updater");
pub const STATE: Item<States> = Item::new("nomad_base_state");
pub const COMMITTED_ROOT: Item<H256> = Item::new("nomad_base_committed_root");
