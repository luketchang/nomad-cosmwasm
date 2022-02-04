use crate::merkle_tree::IncrementalMerkle;

use cw_storage_plus::Item;

pub const MERKLE: Item<IncrementalMerkle> = Item::new("merkle_incremental_merkle");
