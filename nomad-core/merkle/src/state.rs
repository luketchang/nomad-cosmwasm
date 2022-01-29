use crate::merkle_tree::IncrementalMerkle;

use cosmwasm_std::Addr;
use cw_storage_plus::Item;

pub const STATE: Item<IncrementalMerkle> = Item::new("merkle_state");