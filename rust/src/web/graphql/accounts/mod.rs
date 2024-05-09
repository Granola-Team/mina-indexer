use super::db;
use crate::{
    block::store::BlockStore,
    chain_id::{store::ChainIdStore, Network},
    ledger::{account, store::LedgerStore},
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
}

#[derive(InputObject)]
pub struct AccountQueryInput {
    public_key: Option<String>,
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
        let network = db.get_current_network().unwrap_or(Network::Mainnet);
        let ledger = match db.get_ledger_state_hash(&network, &state_hash, true) {
            Ok(Some(ledger)) => ledger,
            Ok(None) | Err(_) => {
                return Ok(None);
            }
        };

        let mut accounts: Vec<Account> = ledger
            .accounts
            .into_values()
            .filter(|account| {
                if let Some(ref query) = query {
                    if let Some(public_key) = &query.public_key {
                        return *public_key == account.public_key.0;
                    }
                    if let Some(balance) = &query.balance {
                        return *balance == account.balance.0;
                    }
                    if let Some(balance_gt) = &query.balance_gt {
                        return *balance_gt < account.balance.0;
                    }
                    if let Some(balance_gte) = &query.balance_gte {
                        return *balance_gte <= account.balance.0;
                    }
                    if let Some(balance_lt) = &query.balance_lt {
                        return *balance_lt > account.balance.0;
                    }
                    if let Some(balance_lte) = &query.balance_lte {
                        return *balance_lte >= account.balance.0;
                    }
                    if let Some(balance_ne) = &query.balance_ne {
                        return *balance_ne != account.balance.0;
                    }
                }
                true
            })
            .map(Account::from)
            .collect();
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

impl From<account::Account> for Account {
    fn from(ledger: account::Account) -> Self {
        Account {
            public_key: ledger.public_key.0,
            delegate: ledger.delegate.0,
            nonce: ledger.nonce.0,
            balance: ledger.balance.0,
            time_locked: false,
            username: ledger.username.map(|u| u.0),
        }
    }
}
