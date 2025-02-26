//! GraphQL `stagedLedgerAccounts` endpoint

use super::db;
use crate::{
    canonicity::store::CanonicityStore,
    ledger::{account::Account, store::staged::StagedLedgerStore, token::TokenAddress},
};
use async_graphql::{Context, Enum, InputObject, Object, Result, SimpleObject};

#[derive(InputObject)]
pub struct StagedLedgerQueryInput {
    ledger_hash: Option<String>,
    state_hash: Option<String>,
    public_key: Option<String>,
    token: Option<String>,

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
        let token = query
            .as_ref()
            .map_or(TokenAddress::default(), |q| match q.token.to_owned() {
                Some(token) => TokenAddress::new(token).expect("valid token address"),
                None => TokenAddress::default(),
            });

        // pk staged account query
        if let Some(pk) = query.as_ref().and_then(|q| q.public_key.clone()) {
            if let Some(state_hash) = query.as_ref().and_then(|q| q.state_hash.clone()) {
                return Ok(db
                    .get_staged_account(&pk.into(), &token, &state_hash.into())?
                    .map(|acct| vec![acct.into()]));
            } else if let Some(ledger_hash) = query.as_ref().and_then(|q| q.ledger_hash.clone()) {
                if let Some(state_hash) =
                    db.get_staged_ledger_block_state_hash(&ledger_hash.into())?
                {
                    return Ok(db
                        .get_staged_account(&pk.into(), &token, &state_hash)?
                        .map(|acct| vec![acct.into()]));
                }
            } else if let Some(block_height) = query.as_ref().and_then(|q| q.blockchain_length) {
                if let Some(state_hash) = db.get_canonical_hash_at_height(block_height)? {
                    return Ok(db
                        .get_staged_account(&pk.into(), &token, &state_hash)?
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
                .tokens
                .get(&token)
                .map(|token_ledger| {
                    token_ledger
                        .accounts
                        .to_owned()
                        .into_values()
                        .map(StagedLedgerAccount::from)
                        .collect()
                })
                .expect("MINA token ledger")
        });

        reorder(&mut accounts, sort_by);
        accounts.truncate(limit);

        Ok(Some(accounts))
    }
}

fn reorder(accts: &mut [StagedLedgerAccount], sort_by: Option<StagedLedgerSortByInput>) {
    if let Some(sort_by) = sort_by {
        match sort_by {
            StagedLedgerSortByInput::BalanceAsc => {
                accts.sort_by_cached_key(|x| (x.balance_nanomina, x.nonce, x.public_key.clone()))
            }
            StagedLedgerSortByInput::BalanceDesc => {
                reorder(accts, Some(StagedLedgerSortByInput::BalanceAsc));
                accts.reverse();
            }
        }
    }
}

impl From<Account> for StagedLedgerAccount {
    fn from(acct: Account) -> Self {
        let balance = acct.balance.display();
        let balance_nanomina = balance.0;

        Self {
            balance_nanomina,
            balance: balance.to_f64(),
            nonce: acct.nonce.map_or(0, |n| n.0),
            delegate: acct.delegate.0,
            public_key: acct.public_key.0,
            username: acct.username.map(|u| u.0),
        }
    }
}
