pub mod delegation_total_store;

use serde::Deserialize;
use serde::Serialize;

use crate::staking_ledger::StakingLedgerAccount;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct DelegationTotal {
    pub count_delegates: i32,
    pub total_delegations: i64,
}

pub fn calculate_delagation_total(
    public_key: &str,
    accounts: Vec<StakingLedgerAccount>,
) -> Option<DelegationTotal> {
    let mut count_delegates = 0;
    let mut total_delegations: i64 = 0;

    for account in accounts.into_iter() {
        if &account.delegate == public_key {
            count_delegates += 1;
            total_delegations += &account.balance.parse::<i64>().unwrap_or(0);
        }
    }

    Some(DelegationTotal {
        count_delegates,
        total_delegations,
    })
}
