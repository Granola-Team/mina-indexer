use super::db;
use crate::ledger::{account::Account, store::LedgerStore};
use async_graphql::{Context, Enum, InputObject, Object, Result, SimpleObject};
use rust_decimal::{prelude::ToPrimitive, Decimal};

#[derive(InputObject)]
pub struct StagedLedgerQueryInput {
    ledger_hash: Option<String>,
    state_hash: Option<String>,

    #[graphql(name = "blockchain_length")]
    blockchain_length: Option<u32>,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum StagedLedgerSortByInput {
    #[graphql(name = "BALANCE_ASC")]
    BalanceAsc,

    #[graphql(name = "BALANCE_DESC")]
    BalanceDesc,
}

#[derive(Default)]
pub struct StagedLedgerQueryRoot;

#[Object]
impl StagedLedgerQueryRoot {
    // Cache for 1 hour
    #[graphql(cache_control(max_age = 3600))]
    async fn staged_ledger_accounts<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        query: Option<StagedLedgerQueryInput>,
        sort_by: Option<StagedLedgerSortByInput>,
        #[graphql(default = 100)] limit: usize,
    ) -> Result<Option<Vec<StagedLedgerAccount>>> {
        let db = db(ctx);

        // ledger hash query
        if let Some(ledger_hash) = query.as_ref().and_then(|q| q.ledger_hash.clone()) {
            let mut accounts: Vec<StagedLedgerAccount> = db
                .get_ledger(&ledger_hash.into())?
                .map_or(vec![], |ledger| {
                    ledger
                        .accounts
                        .into_values()
                        .map(<Account as Into<StagedLedgerAccount>>::into)
                        .collect()
                });

            reorder(&mut accounts, sort_by);
            accounts.truncate(limit);
            return Ok(Some(accounts));
        }

        // state hash query
        if let Some(state_hash) = query.as_ref().and_then(|q| q.state_hash.clone()) {
            let mut accounts: Vec<StagedLedgerAccount> = db
                .get_ledger_state_hash(&state_hash.into(), true)?
                .map_or(vec![], |ledger| {
                    ledger
                        .accounts
                        .into_values()
                        .map(<Account as Into<StagedLedgerAccount>>::into)
                        .collect()
                });

            reorder(&mut accounts, sort_by);
            accounts.truncate(limit);
            return Ok(Some(accounts));
        }

        // blockchain length query
        if let Some(blockchain_length) = query.as_ref().and_then(|q| q.blockchain_length) {
            let mut accounts: Vec<StagedLedgerAccount> = db
                .get_ledger_at_height(blockchain_length, true)?
                .map_or(vec![], |ledger| {
                    ledger
                        .accounts
                        .into_values()
                        .map(<Account as Into<StagedLedgerAccount>>::into)
                        .collect()
                });

            reorder(&mut accounts, sort_by);
            accounts.truncate(limit);
            return Ok(Some(accounts));
        }

        Ok(None)
    }
}

fn reorder(accts: &mut [StagedLedgerAccount], sort_by: Option<StagedLedgerSortByInput>) {
    match sort_by {
        Some(StagedLedgerSortByInput::BalanceAsc) => {
            accts.sort_by_cached_key(|x| (x.balance_nanomina, x.public_key.clone()))
        }
        Some(StagedLedgerSortByInput::BalanceDesc) => {
            reorder(accts, Some(StagedLedgerSortByInput::BalanceAsc));
            accts.reverse();
        }
        None => (),
    }
}

#[derive(SimpleObject)]
pub struct StagedLedgerWithMeta {
    /// Value blockchain length
    #[graphql(name = "blockchain_length")]
    blockchain_length: u32,

    /// Value state hash
    state_hash: String,

    /// Value ledger hash
    ledger_hash: String,

    /// Value staged ledger accounts
    accounts: Vec<StagedLedgerAccount>,
}

#[derive(SimpleObject)]
pub struct StagedLedgerAccount {
    /// Value public key
    #[graphql(name = "public_key")]
    pub public_key: String,

    /// Value delegate
    pub delegate: String,

    /// Value balance
    pub balance: f64,

    /// Value balance
    #[graphql(name = "balance_nanomina")]
    pub balance_nanomina: u64,

    /// Value nonce
    pub nonce: u32,

    /// Value username
    pub username: Option<String>,
}

impl From<Account> for StagedLedgerAccount {
    fn from(acct: Account) -> Self {
        let balance_nanomina = acct.balance.0;
        let mut decimal = Decimal::from(balance_nanomina);
        decimal.set_scale(9).ok();

        Self {
            nonce: acct.nonce.map_or(0, |n| n.0),
            delegate: acct.delegate.0,
            public_key: acct.public_key.0,
            username: acct.username.map(|u| u.0),
            balance: decimal.to_f64().unwrap_or_default(),
            balance_nanomina: decimal.to_u64().unwrap_or_default(),
        }
    }
}
