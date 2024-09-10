use super::{db, MAINNET_ACCOUNT_CREATION_FEE};
use crate::{
    canonicity::store::CanonicityStore,
    ledger::{
        account::{Account, Amount},
        store::staged::StagedLedgerStore,
    },
};
use async_graphql::{Context, Enum, InputObject, Object, Result, SimpleObject};
use rust_decimal::{prelude::ToPrimitive, Decimal};

#[derive(InputObject)]
pub struct StagedLedgerQueryInput {
    ledger_hash: Option<String>,
    state_hash: Option<String>,
    public_key: Option<String>,

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

        // pk staged account query
        if let Some(pk) = query.as_ref().and_then(|q| q.public_key.clone()) {
            if let Some(state_hash) = query.as_ref().and_then(|q| q.state_hash.clone()) {
                return Ok(db
                    .get_staged_account(pk.into(), state_hash.into())?
                    .map(|acct| vec![acct.into()]));
            } else if let Some(ledger_hash) = query.as_ref().and_then(|q| q.ledger_hash.clone()) {
                if let Some(state_hash) =
                    db.get_staged_ledger_block_state_hash(&ledger_hash.into())?
                {
                    return Ok(db
                        .get_staged_account(pk.into(), state_hash)?
                        .map(|acct| vec![acct.into()]));
                }
            } else if let Some(block_height) = query.as_ref().and_then(|q| q.blockchain_length) {
                if let Some(state_hash) = db.get_canonical_hash_at_height(block_height)? {
                    return Ok(db
                        .get_staged_account(pk.into(), state_hash)?
                        .map(|acct| vec![acct.into()]));
                }
            }
            return Ok(None);
        }

        // otherwise build the staged ledger from
        // - block state hash
        // - staged ledger hash
        // - canonical block height
        let staged_ledger =
            if let Some(state_hash) = query.as_ref().and_then(|q| q.state_hash.clone()) {
                db.get_staged_ledger_at_state_hash(&state_hash.into(), false)?
            } else if let Some(ledger_hash) = query.as_ref().and_then(|q| q.ledger_hash.clone()) {
                db.get_staged_ledger_at_ledger_hash(&ledger_hash.into(), false)?
            } else if let Some(block_height) = query.as_ref().and_then(|q| q.blockchain_length) {
                db.get_staged_ledger_at_block_height(block_height, false)?
            } else {
                return Ok(None);
            };
        let mut accounts = staged_ledger.map_or(vec![], |ledger| {
            ledger
                .accounts
                .into_values()
                .map(StagedLedgerAccount::from)
                .collect()
        });

        reorder(&mut accounts, sort_by);
        accounts.truncate(limit);
        Ok(Some(accounts))
    }
}

fn reorder(accts: &mut [StagedLedgerAccount], sort_by: Option<StagedLedgerSortByInput>) {
    match sort_by {
        Some(StagedLedgerSortByInput::BalanceAsc) => {
            accts.sort_by_cached_key(|x| (x.balance_nanomina, x.nonce, x.public_key.clone()))
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
        // deduct 1 MINA fee for display
        let balance_nanomina = acct.balance - Amount::new(MAINNET_ACCOUNT_CREATION_FEE);
        let mut decimal = Decimal::from(balance_nanomina.value());
        decimal.set_scale(9).ok();

        Self {
            balance_nanomina: balance_nanomina.value(),
            nonce: acct.nonce.map_or(0, |n| n.0),
            delegate: acct.delegate.0,
            public_key: acct.public_key.0,
            username: acct.username.map(|u| u.0),
            balance: decimal.to_f64().unwrap_or_default(),
        }
    }
}
