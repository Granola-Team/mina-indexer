use crate::mina_blocks::v2;
use mina_serialization_proc_macros::AutoFrom;
use serde::{Deserialize, Serialize};

#[derive(
    Default, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, AutoFrom,
)]
#[auto_from(v2::Timing)]
pub struct Timing {
    pub initial_minimum_balance: u64,
    pub cliff_time: u32,
    pub cliff_amount: u64,
    pub vesting_period: u32,
    pub vesting_increment: u64,
}
