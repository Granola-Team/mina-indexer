use juniper::GraphQLInputObject;
use serde::{Deserialize, Serialize};

use crate::{
    gql::root::Context,
    staking_ledger::{
        staking_ledger_store::StakingLedgerStore, StakingLedger, StakingLedgerAccount,
    },
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
}

pub fn get_accounts(ctx: &Context, query: Option<StakesQueryInput>) -> Vec<StakingLedgerAccount> {
    let mut accounts: Vec<StakingLedgerAccount> = Vec::new();
    if let Some(ref query_input) = query {
        if let Some(epoch) = query_input.epoch {
            accounts = ctx
                .db
                .get_epoch(epoch as u32)
                .unwrap_or(None)
                .map(|ledger| Stakes::from_staking_ledger(&ledger))
                .map(|foo| foo.accounts)
                .unwrap();
        }
    }
    accounts
}

#[juniper::graphql_object(Context = Context)]
#[graphql(description = "Stakes")]
impl StakingLedgerAccount {
    #[graphql(description = "Epoch Number")]
    fn epoch(&self) -> i32 {
        let epoch = &self.epoch_number.unwrap();
        return epoch.clone();
    }
}

#[derive(GraphQLInputObject)]
#[graphql(description = "Stakes query input")]
pub struct StakesQueryInput {
    pub epoch: Option<i32>,
    pub public_key: Option<String>,
}
