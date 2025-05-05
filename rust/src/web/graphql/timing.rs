//! GQL timing

use async_graphql::SimpleObject;

#[derive(SimpleObject)]
pub struct Timing {
    #[graphql(name = "cliff_amount")]
    pub cliff_amount: Option<u64>,

    #[graphql(name = "cliff_time")]
    pub cliff_time: Option<u32>,

    #[graphql(name = "initial_minimum_balance")]
    pub initial_minimum_balance: Option<u64>,

    #[graphql(name = "vesting_period")]
    pub vesting_period: Option<u32>,

    #[graphql(name = "vesting_increment")]
    pub vesting_increment: Option<u64>,
}
