use std::collections::VecDeque;

use cw_storage_plus::Item;

pub const QUEUE: Item<VecDeque<[u8; 32]>> = Item::new("queue");
