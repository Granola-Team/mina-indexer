//! GraphQL `stagedLedgerAccounts` endpoint

use super::{
    db,
    pk::{DelegatePK, PK},
};
use crate::{
    base::{public_key::PublicKey, state_hash::StateHash},
    canonicity::store::CanonicityStore,
    ledger::{account::Account, store::staged::StagedLedgerStore, token::TokenAddress, LedgerHash},
    store::IndexerStore,
};
use anyhow::Context as aContext;
use async_graphql::{Context, Enum, InputObject, Object, Result, SimpleObject};

#[derive(InputObject)]
pub struct StagedLedgerQueryInput {
    /// Input staged ledger hash
    ledger_hash: Option<String>,

    /// Input block state hash
    state_hash: Option<String>,

    /// Input account public key
    public_key: Option<String>,

    /// Input account token
    token: Option<String>,

    /// Input blockchain length (aka block height)
    #[graphql(name = "blockchain_length")]
    blockchain_length: Option<u32>,
}

#[derive(Default, Enum, Copy, Clone, Eq, PartialEq)]
pub enum StagedLedgerSortByInput {
    #[default]
    BalanceDesc,
    BalanceAsc,
}

#[derive(SimpleObject)]
pub struct StagedLedgerWithMeta {
    /// Value blockchain length (aka block height)
    #[graphql(name = "blockchain_length")]
    blockchain_length: u32,

    /// Value block state hash
    state_hash: String,

    /// Value staged ledger hash
    ledger_hash: String,

    /// Value staged ledger accounts
    accounts: Vec<StagedLedgerAccount>,
}

#[derive(SimpleObject)]
pub struct StagedLedgerAccount {
    /// Value account public key/username
    #[graphql(flatten)]
    pub public_key: PK,

    /// Value delegate
    #[graphql(flatten)]
    pub delegate: DelegatePK,

    /// Value balance
    pub balance: f64,

    /// Value balance (nano)
    #[graphql(name = "balance_nano")]
    pub balance_nano: u64,

    /// Value nonce
    pub nonce: u32,

    /// Value token address
    pub token: String,
}

#[derive(Default)]
pub struct StagedLedgerQueryRoot;

#[Object]
impl StagedLedgerQueryRoot {
    // Cache for 1 hour
    #[graphql(cache_control(max_age = 3600))]
    async fn staged_ledger_accounts(
        &self,
        ctx: &Context<'_>,
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
        if let Some(pk) = query.as_ref().and_then(|q| q.public_key.as_ref()) {
            // validate public key
            let pk = match PublicKey::new(pk) {
                Ok(pk) => pk,
                Err(_) => {
                    return Err(async_graphql::Error::new(format!(
                        "Invalid public key: {}",
                        pk,
                    )))
                }
            };

            if let Some(state_hash) = query.as_ref().and_then(|q| q.state_hash.as_ref()) {
                // validate state hash
                let state_hash = match StateHash::new(state_hash) {
                    Ok(state_hash) => state_hash,
                    Err(_) => {
                        return Err(async_graphql::Error::new(format!(
                            "Invalid state hash: {}",
                            state_hash,
                        )))
                    }
                };

                return Ok(db
                    .get_staged_account(&pk, &token, &state_hash)?
                    .map(|acct| vec![StagedLedgerAccount::new(db, acct)]));
            } else if let Some(ledger_hash) = query.as_ref().and_then(|q| q.ledger_hash.as_ref()) {
                // validate ledger hash
                let ledger_hash = match LedgerHash::new(ledger_hash) {
                    Ok(ledger_hash) => ledger_hash,
                    Err(_) => {
                        return Err(async_graphql::Error::new(format!(
                            "Invalid ledger hash: {}",
                            ledger_hash
                        )))
                    }
                };

                if let Some(state_hash) = db
                    .get_staged_ledger_block_state_hash(&ledger_hash)?
                    .as_ref()
                {
                    return Ok(db
                        .get_staged_account(&pk, &token, state_hash)?
                        .map(|acct| vec![StagedLedgerAccount::new(db, acct)]));
                }
            } else if let Some(block_height) = query.as_ref().and_then(|q| q.blockchain_length) {
                if let Some(state_hash) = db.get_canonical_hash_at_height(block_height)?.as_ref() {
                    return Ok(db
                        .get_staged_account(&pk, &token, state_hash)?
                        .map(|acct| vec![StagedLedgerAccount::new(db, acct)]));
                }
            }

            return Ok(None);
        }

        // otherwise build the staged ledger from
        // - block state hash
        // - staged ledger hash
        // - canonical block height
        let staged_ledger =
            if let Some(state_hash) = query.as_ref().and_then(|q| q.state_hash.as_ref()) {
                // validate state hash
                let state_hash = match StateHash::new(state_hash) {
                    Ok(state_hash) => state_hash,
                    Err(_) => {
                        return Err(async_graphql::Error::new(format!(
                            "Invalid state hash: {}",
                            state_hash,
                        )))
                    }
                };

                db.get_staged_ledger_at_state_hash(&state_hash, false)?
            } else if let Some(ledger_hash) = query.as_ref().and_then(|q| q.ledger_hash.as_ref()) {
                // validate ledger hash
                let ledger_hash = match LedgerHash::new(ledger_hash) {
                    Ok(ledger_hash) => ledger_hash,
                    Err(_) => {
                        return Err(async_graphql::Error::new(format!(
                            "Invalid ledger hash: {}",
                            ledger_hash,
                        )))
                    }
                };

                db.get_staged_ledger_at_ledger_hash(&ledger_hash, false)?
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
                        .values()
                        .cloned()
                        .map(|account| StagedLedgerAccount::new(db, account))
                        .collect()
                })
                .with_context(|| format!("token {}", token))
                .expect("token ledger")
        });

        reorder(&mut accounts, sort_by);
        accounts.truncate(limit);

        Ok(Some(accounts))
    }
}

impl StagedLedgerAccount {
    /// Account creation fee deducted here
    fn new(db: &std::sync::Arc<IndexerStore>, account: Account) -> Self {
        let account = account.deduct_mina_account_creation_fee();

        Self {
            balance_nano: account.balance.0,
            balance: account.balance.to_f64(),
            nonce: account.nonce.map_or(0, |n| n.0),
            delegate: DelegatePK::new(db, account.delegate.0),
            public_key: PK::new(db, account.public_key),
            token: account.token.unwrap_or_default().0,
        }
    }
}

/////////////
// helpers //
/////////////

fn reorder(accts: &mut [StagedLedgerAccount], sort_by: Option<StagedLedgerSortByInput>) {
    if let Some(sort_by) = sort_by {
        match sort_by {
            StagedLedgerSortByInput::BalanceAsc => accts
                .sort_by_cached_key(|x| (x.balance_nano, x.nonce, x.public_key.public_key.clone())),
            StagedLedgerSortByInput::BalanceDesc => {
                reorder(accts, Some(StagedLedgerSortByInput::BalanceAsc));
                accts.reverse();
            }
        }
    }
}
