use cw_storage_plus::Item;
use ethers_core::types::H256;
use std::collections::VecDeque;

pub const QUEUE: Item<VecDeque<H256>> = Item::new("queue_queue");
