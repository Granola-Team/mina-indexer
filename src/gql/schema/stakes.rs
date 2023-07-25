use juniper::GraphQLInputObject;
use serde::{Deserialize, Serialize};

use crate::{
    gql::root::Context,
    staking_ledger::{StakingLedger, StakingLedgerAccount},
};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Stakes {
    pub epoch_number: i32,
    pub ledger_hash: String,
    pub accounts: Vec<StakingLedgerAccount>,
}

impl Stakes {
    pub fn from_staking_ledger(ledger: &StakingLedger) -> Self {
        Self {
            epoch_number: ledger.epoch_number,
            ledger_hash: ledger.ledger_hash.clone(),
            accounts: ledger.accounts.clone(),
        }
    }

    pub fn to_camel_case(&self) -> CamelCasedStakes {
        CamelCasedStakes::to_camel_case(self)
    }
}

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash)]
pub struct CamelCasedStakes {
    pub epochNumber: i32,
    pub ledgerHash: String,
    pub accounts: Vec<StakingLedgerAccount>,
}

impl CamelCasedStakes {
    pub fn to_camel_case(ledger: &Stakes) -> CamelCasedStakes {
        CamelCasedStakes {
            epochNumber: ledger.epoch_number,
            ledgerHash: ledger.ledger_hash.clone(),
            accounts: ledger.accounts.clone(),
        }
    }
}

#[juniper::graphql_object(Context = Context)]
#[graphql(description = "Stakes")]
impl CamelCasedStakes {
    #[graphql(description = "Epoch Number")]
    fn epochNumber(&self) -> &i32 {
        &self.epochNumber
    }

    #[graphql(description = "Ledger Hash")]
    fn ledgerHash(&self) -> &str {
        &self.ledgerHash
    }
}

#[derive(GraphQLInputObject)]
#[graphql(description = "Stakes query input")]
pub struct CamelCasedStakesQueryInput {
    pub epoch_number: Option<i32>,
    pub ledger_hash: Option<String>,
}
