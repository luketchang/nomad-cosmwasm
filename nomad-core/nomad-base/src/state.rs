use ethers_core::types::H256;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Addr;
use cw_storage_plus::Item;

/// States:
///   0 - UnInitialized - before initialize function is called
///   note: the contract is initialized at deploy time, so it should never be in
///   this state
///   1 - Active - as long as the contract has not become fraudulent
///   2 - Failed - after a valid fraud proof has been submitted;
///   contract will no longer accept updates or new messages
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum States {
    UnInitialized,
    Active,
    Failed,
}

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