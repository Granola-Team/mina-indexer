use crate::{
    block::store::BlockStore,
    ledger::{account::Account as LAccount, store::LedgerStore},
    store::IndexerStore,
};
use async_graphql::{Context, Enum, InputObject, Object, Result, SimpleObject};
use std::sync::Arc;

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
    public_key: String,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum AccountSortByInput {
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
        limit: Option<usize>,
    ) -> Result<Option<Vec<Account>>> {
        let db = ctx
            .data::<Arc<IndexerStore>>()
            .expect("db to be in context");
        let limit = limit.unwrap_or(100);
        let state_hash = match db.get_best_block_hash() {
            Ok(Some(state_hash)) => state_hash,
            Ok(None) => return Ok(None),
            Err(_) => {
                return Ok(None);
            }
        };
        let ledger = match db.get_ledger_state_hash("mainnet", &state_hash, true) {
            Ok(Some(ledger)) => ledger,
            Ok(None) => return Ok(None),
            Err(_) => {
                return Ok(None);
            }
        };

        let mut accounts: Vec<Account> = ledger
            .accounts
            .into_values()
            .filter(|account| {
                if let Some(ref query) = query {
                    return *query.public_key == account.public_key.0;
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
            }
        }

        accounts.truncate(limit);
        Ok(Some(accounts))
    }
}

impl From<LAccount> for Account {
    fn from(ledger: LAccount) -> Self {
        Account {
            public_key: ledger.public_key.0,
            delegate: ledger.delegate.0,
            nonce: ledger.nonce.0,
            balance: ledger.balance.0,
            username: None,
            time_locked: false,
        }
    }
}
