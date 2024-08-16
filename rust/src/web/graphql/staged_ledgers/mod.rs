use super::db;
use crate::{
    canonicity::store::CanonicityStore,
    ledger::{
        account::Account,
        store::staged::{split_staged_account_balance_sort_key, StagedLedgerStore},
    },
};
use anyhow::Context as aContext;
use async_graphql::{Context, Enum, InputObject, Object, Result, SimpleObject};
use log::error;
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
        let direction = match sort_by {
            Some(StagedLedgerSortByInput::BalanceDesc) | None => speedb::Direction::Reverse,
            Some(StagedLedgerSortByInput::BalanceAsc) => speedb::Direction::Forward,
        };
        let (ledger_state_hash, iter) = {
            if let Some(ledger_hash) = query.as_ref().and_then(|q| q.ledger_hash.clone()) {
                if let Some(state_hash) =
                    db.get_staged_ledger_block_state_hash(&ledger_hash.clone().into())?
                {
                    (
                        state_hash.clone(),
                        db.staged_ledger_account_balance_iterator(&state_hash, direction),
                    )
                } else {
                    error!("Missing block corresponding to staged ledger {ledger_hash}");
                    return Ok(None);
                }
            } else if let Some(state_hash) = query.as_ref().and_then(|q| q.state_hash.clone()) {
                (
                    state_hash.clone().into(),
                    db.staged_ledger_account_balance_iterator(&state_hash.into(), direction),
                )
            } else if let Some(blockchain_length) = query.as_ref().and_then(|q| q.blockchain_length)
            {
                if let Some(state_hash) = db.get_canonical_hash_at_height(blockchain_length)? {
                    (
                        state_hash.clone(),
                        db.staged_ledger_account_balance_iterator(&state_hash, direction),
                    )
                } else {
                    error!("Missing block at height {blockchain_length}");
                    return Ok(None);
                }
            } else {
                return Ok(None);
            }
        };

        let mut accounts = vec![];
        for (key, _) in iter.flatten() {
            if let Some((state_hash, _, pk)) = split_staged_account_balance_sort_key(&key) {
                if ledger_state_hash != state_hash || accounts.len() >= limit {
                    break;
                }

                let account = db
                    .get_staged_account(pk.clone(), state_hash.clone())?
                    .with_context(|| format!("staged account {pk}, state hash {state_hash}"))
                    .expect("account exists");
                accounts.push(account.into());
            }
        }
        Ok(Some(accounts))
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
