use juniper::GraphQLInputObject;
use serde::{Deserialize, Serialize};

use crate::{
    delegation_totals_store::{self, get_delegation_totals_from_db, update_delegation_totals},
    gql::{
        root::Context,
        schema::delegations::{DelegationTotals, TotalDelegated},
    },
    staking_ledger::{
        staking_ledger_store::StakingLedgerStore, StakingLedger, StakingLedgerAccount,
    },
    state::ledger::public_key,
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
        *epoch
    }
    #[graphql(description = "Public Key")]
    fn public_key(&self) -> &str {
        &self.pk
    }
    #[graphql(description = "Delegate Key")]
    fn delegate(&self) -> &str {
        &self.delegate
    }
    #[graphql(description = "Account Balance")]
    fn balance(&self) -> f64 {
        self.balance.parse::<f64>().unwrap()
    }
    #[graphql(description = "Delegation Totals")]
    fn delegation_totals(&self) -> Option<DelegationTotals> {
        // delegation totals for the default epoch (1) here
        let epoch_number = 1;
        let public_key = &self.pk; // Use a public key from genesis as default?

        let mut total_delegated = TotalDelegated(0.0);
        let mut count_delegates = 0;

        if let Some(staking_ledger) = staking_ledger {
            for account in staking_ledger.accounts {
                if let Some(delegation_totals) =
                    get_delegation_totals_from_db(&delegation_totals_db, &account.pk, epoch_number)
                        .expect("Failed to fetch delegation totals")
                {
                    total_delegated.0 += delegation_totals.total_delegated.0;
                    count_delegates += delegation_totals.count_delegates;
                }
            }
        }

        update_delegation_totals(
            &delegation_totals_db,
            "public_key_here", // placeholder code for public key
            epoch_number,
            total_delegated,
            count_delegates,
        )
        .expect("Failed to update delegation totals");
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
