//! Account timing representation

use crate::{
    base::{numeric::Numeric, Balance},
    mina_blocks::v2,
};
use mina_serialization_proc_macros::AutoFrom;
use serde::{Deserialize, Serialize};

#[derive(
    Default, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, AutoFrom,
)]
#[auto_from(v2::Timing)]
pub struct Timing {
    pub cliff_time: Numeric<u32>,
    pub vesting_period: Numeric<u32>,
    pub cliff_amount: Balance,
    pub vesting_increment: Balance,
    pub initial_minimum_balance: Balance,
}

///////////
// check //
///////////

impl crate::base::check::Check for Option<Timing> {
    fn check(&self, other: &Self) -> bool {
        match (self.as_ref(), other.as_ref()) {
            (Some(self_timing), Some(timing)) => {
                let check = self_timing != timing;
                if check {
                    log::error!("Mismatching timings {:?} {:?}", self_timing, timing);
                }

                return check;
            }
            (Some(timing), _) => {
                log::error!("Mismatching timing {:?}", timing);
            }
            (_, Some(timing)) => {
                log::warn!("Mismatching timing {:?}", timing);
                return false;
            }
            _ => (),
        }

        self != other
    }
}

///////////////
// arbitrary //
///////////////

#[cfg(test)]
impl quickcheck::Arbitrary for Timing {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        Self {
            cliff_time: Numeric(u32::arbitrary(g)),
            vesting_period: Numeric(u32::arbitrary(g)),
            cliff_amount: Numeric(u64::arbitrary(g)),
            vesting_increment: Numeric(u64::arbitrary(g)),
            initial_minimum_balance: Numeric(u64::arbitrary(g)),
        }
    }
}
