use super::db;
use crate::{
    block::store::BlockStore,
    ledger::{account, store::LedgerStore},
    web::graphql::Timing,
};
use async_graphql::{Context, Enum, InputObject, Object, Result, SimpleObject};

#[derive(SimpleObject)]
pub struct Account {
    public_key: String,
    username: Option<String>,
    delegate: String,
    balance: u64,
    nonce: u32,
    time_locked: bool,
    timing: Option<Timing>,
}

#[derive(InputObject)]
pub struct AccountQueryInput {
    public_key: Option<String>,
    // username: Option<String>,
    balance: Option<u64>,

    #[graphql(name = "balance_gt")]
    balance_gt: Option<u64>,

    #[graphql(name = "balance_gte")]
    balance_gte: Option<u64>,

    #[graphql(name = "balance_lt")]
    balance_lt: Option<u64>,

    #[graphql(name = "balance_lte")]
    balance_lte: Option<u64>,

    #[graphql(name = "balance_ne")]
    balance_ne: Option<u64>,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum AccountSortByInput {
    BalanceAsc,
    BalanceDesc,
}

#[derive(Default)]
pub struct AccountQueryRoot;

#[Object]
impl AccountQueryRoot {
    async fn accounts<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        query: Option<AccountQueryInput>,
        sort_by: Option<AccountSortByInput>,
        #[graphql(default = 100)] limit: usize,
    ) -> Result<Option<Vec<Account>>> {
        let db = db(ctx);
        let state_hash = match db.get_best_block_hash() {
            Ok(Some(state_hash)) => state_hash,
            Ok(None) | Err(_) => {
                return Ok(None);
            }
        };
        let ledger = match db.get_ledger_state_hash(&state_hash, true) {
            Ok(Some(ledger)) => ledger,
            Ok(None) | Err(_) => {
                return Ok(None);
            }
        };

        // public key query handler
        if let Some(public_key) = query.as_ref().and_then(|q| q.public_key.clone()) {
            return Ok(ledger
                .accounts
                .get(&public_key.into())
                .filter(|acct| query.unwrap().matches(acct))
                .map(|acct| vec![Account::from(acct.clone())]));
        }

        // TODO default query handler use balance-sorted accounts
        let mut accounts: Vec<Account> = if let Some(query) = query {
            ledger
                .accounts
                .into_values()
                .filter(|account| query.matches(account))
                .map(Account::from)
                .collect()
        } else {
            ledger.accounts.into_values().map(Account::from).collect()
        };

        if let Some(sort_by) = sort_by {
            match sort_by {
                AccountSortByInput::BalanceDesc => {
                    accounts.sort_by(|a, b| b.balance.cmp(&a.balance));
                }
                AccountSortByInput::BalanceAsc => {
                    accounts.sort_by(|a, b| a.balance.cmp(&b.balance));
                }
            }
        }

        accounts.truncate(limit);
        Ok(Some(accounts))
    }
}

impl AccountQueryInput {
    fn matches(&self, account: &account::Account) -> bool {
        let AccountQueryInput {
            public_key,
            // username,
            balance,
            balance_gt,
            balance_gte,
            balance_lt,
            balance_lte,
            balance_ne,
        } = self;
        if let Some(public_key) = public_key {
            return *public_key == account.public_key.0;
        }
        // if let Some(username) = username {
        //     return account
        //         .username
        //         .as_ref()
        //         .map_or(false, |u| *username == u.0);
        // }
        if let Some(balance) = balance {
            return *balance == account.balance.0;
        }
        if let Some(balance_gt) = balance_gt {
            return *balance_gt < account.balance.0;
        }
        if let Some(balance_gte) = balance_gte {
            return *balance_gte <= account.balance.0;
        }
        if let Some(balance_lt) = balance_lt {
            return *balance_lt > account.balance.0;
        }
        if let Some(balance_lte) = balance_lte {
            return *balance_lte >= account.balance.0;
        }
        if let Some(balance_ne) = balance_ne {
            return *balance_ne != account.balance.0;
        }
        true
    }
}

impl From<account::Account> for Account {
    fn from(account: account::Account) -> Self {
        Self {
            public_key: account.public_key.0,
            delegate: account.delegate.0,
            nonce: account.nonce.0,
            balance: account.balance.0,
            time_locked: account.timing.is_some(),
            timing: account.timing.map(|t| t.into()),
            username: account.username.map(|u| u.0),
        }
    }
}

impl From<account::Timing> for Timing {
    fn from(timing: account::Timing) -> Self {
        Self {
            initial_minimum_balance: Some(timing.initial_minimum_balance),
            cliff_time: Some(timing.cliff_time),
            cliff_amount: Some(timing.cliff_amount),
            vesting_period: Some(timing.vesting_period),
            vesting_increment: Some(timing.vesting_increment),
        }
    }
}
