use std::collections::VecDeque;
use ethers_core::types::H256;
use cw_storage_plus::Item;

pub const QUEUE: Item<VecDeque<H256>> = Item::new("queue");
