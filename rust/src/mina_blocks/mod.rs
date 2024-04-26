pub mod v1;

use crate::block::BlockHash;

pub trait MinaBlock {
    fn state_hash(&self) -> BlockHash;

    fn blockchain_length(&self) -> u32;
}
