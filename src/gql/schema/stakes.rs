use juniper::GraphQLInputObject;

use crate::{
    gql::root::Context,
    staking_ledger::{StakingLedger, StakingLedgerAccount},
};

#[allow(non_snake_case)]
pub struct Stakes {
    pub epochNumber: i32,
    pub ledgerHash: String,
    pub accounts: Vec<StakingLedgerAccount>,
}

impl Stakes {
    pub fn from_staking_ledger(ledger: &StakingLedger) -> Self {
        Self {
            epochNumber: ledger.epochNumber,
            ledgerHash: ledger.ledgerHash.clone(),
            accounts: ledger.accounts.clone(),
        }
    }
}

#[juniper::graphql_object(Context = Context)]
#[graphql(description = "Stakes")]
impl Stakes {
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
pub struct StakesQueryInput {
    pub epoch_number: Option<i32>,
    pub ledger_hash: Option<String>,
}
