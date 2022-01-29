use std::collections::VecDeque;

use cw_storage_plus::Item;
use lib::Bytes32;

pub const STATE: Item<VecDeque<Bytes32>> = Item::new("queue_state");
