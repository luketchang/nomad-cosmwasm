use crate::{traits::TypedMessage, TypedView};
use ethers_core::{types::H256, utils::keccak256};
use lazy_static::__Deref;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;

const BATCH_MESSAGE_LEN: usize = 33;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Call {
    to: H256,
    data: Vec<u8>,
}

#[repr(u8)]
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub enum GovTypes {
    Invalid = 0,
    Batch = 1,
    TransferGovernor = 2,
}

impl From<u8> for GovTypes {
    fn from(num: u8) -> Self {
        match num {
            0 => Self::Invalid,
            1 => Self::Batch,
            2 => Self::TransferGovernor,
            _ => panic!("Invalid u8 for GovernanceMessage enum!"),
        }
    }
}

pub type GovernanceMessage = TypedView;
impl TypedMessage for GovernanceMessage {
    type MessageEnum = GovTypes;
}

impl GovernanceMessage {
    /* Batch: batch of calls to execute
     * type (1 byte) || hash (32 bytes)
     */

    /// Format a `batch` governance call
    pub fn format_batch(calls: Vec<Call>) -> Self {
        let mut buf: Vec<u8> = Vec::new();

        buf.push(GovTypes::Batch as u8);
        buf.extend(Self::get_batch_hash(calls).as_bytes().to_vec());
        GovernanceMessage::new(buf)
    }

    /// Format a call as: to || data_len || data
    pub fn serialize_call(call: Call) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::new();
        buf.extend(call.to.as_bytes().to_vec());
        buf.extend((call.data.len() as i32).to_be_bytes().to_vec());
        buf.extend(call.data);

        buf
    }

    /// Hash of the following: num_calls || call_1 || call_2 || ... || call_n
    pub fn get_batch_hash(calls: Vec<Call>) -> H256 {
        let mut batch: Vec<Vec<u8>> = Vec::new();
        batch.push(vec![calls.len().try_into().unwrap()]);

        for call in calls {
            batch.push(Self::serialize_call(call));
        }

        let flattened_batch = batch.into_iter().flatten().collect::<Vec<u8>>();
        keccak256(flattened_batch).into()
    }

    /// Checks if batch call has the proper prefix and length
    pub fn is_valid_batch(&self) -> bool {
        self.message_type() == GovTypes::Batch && self.len() == BATCH_MESSAGE_LEN
    }

    /// Retrieve batch hash from formatted batch message
    pub fn batch_hash(&self) -> H256 {
        H256::from_slice(&self[1..])
    }
}
