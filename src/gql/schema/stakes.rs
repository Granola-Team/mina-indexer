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
    pub epoch_number: u32,
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

pub fn get_accounts(
    ctx: &Context,
    query: Option<StakesQueryInput>,
    limit: Option<i32>,
) -> Vec<StakingLedgerAccount> {
    let limit = limit.unwrap_or(100);
    let limit_idx = limit as usize;

    let mut raw_accounts: Vec<StakingLedgerAccount> = Vec::new();
    if let Some(ref query_input) = query {
        if let Some(epoch) = query_input.epoch {
            let ledger = ctx.db.get_epoch(epoch as u32);
            raw_accounts = ledger
                .unwrap()
                .map(|ledger| Stakes::from_staking_ledger(&ledger))
                .map(|stakes: Stakes| stakes.accounts)
                .unwrap();
        }
    }
    let mut accounts: Vec<StakingLedgerAccount> = Vec::new();
    for account in raw_accounts {
        // If query is provided, only add accounts that satisfy the query
        if let Some(ref query_input) = query {
            if query_input.matches(&account) {
                accounts.push(account);
            }
        }
        // If no query is provided, add all transactions
        else {
            accounts.push(account);
        }
        // Early break if the transactions reach the query limit
        if accounts.len() >= limit_idx {
            break;
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
    #[graphql(description = "Public Key")]
    fn public_key(&self) -> &str {
        &self.pk
    }
    #[graphql(description = "Delegate Key")]
    fn delegate(&self) -> &str {
        &self.delegate
    }
    #[graphql(description = "Account balance")]
    fn balance(&self) -> f64 {
        self.balance.parse::<f64>().unwrap()
    }
}

#[derive(GraphQLInputObject)]
#[graphql(description = "Stakes query input")]
pub struct StakesQueryInput {
    pub epoch: Option<i32>,
    pub public_key: Option<String>,
    // Logical  operators
    #[graphql(name = "OR")]
    pub or: Option<Vec<StakesQueryInput>>,
    #[graphql(name = "AND")]
    pub and: Option<Vec<StakesQueryInput>>,
}

impl StakesQueryInput {
    fn matches(&self, account: &StakingLedgerAccount) -> bool {
        let mut matches = true;

        if let Some(ref public_key) = self.public_key {
            matches = matches && account.pk == *public_key;
        }

        if let Some(ref query) = self.and {
            matches = matches && query.iter().all(|and| and.matches(account));
        }

        if let Some(ref query) = self.or {
            if !query.is_empty() {
                matches = matches && query.iter().any(|or| or.matches(account));
            }
        }
        matches
    }
}
