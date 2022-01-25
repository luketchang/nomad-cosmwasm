use schemars::JsonSchema;
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
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum States {
    UnInitialized,
    Active,
    Failed
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct State {
    pub local_domain: u32,
    pub updater: Addr,
    pub state: States,
    pub committed_root: [u8; 32],
}

pub const STATE: Item<State> = Item::new("state");
