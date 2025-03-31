//! GraphQL `accounts` endpoint

mod zkapp;

use super::db;
use crate::{
    base::public_key::PublicKey,
    block::store::BlockStore,
    command::{internal::store::InternalCommandStore, store::UserCommandStore},
    constants::MINA_TOKEN_ADDRESS,
    ledger::{account, store::best::BestLedgerStore, token::TokenAddress},
    snark_work::store::SnarkStore,
    store::{username::UsernameStore, IndexerStore},
    utility::store::common::U64_LEN,
    web::graphql::Timing,
};
use async_graphql::{Context, Enum, InputObject, Object, Result, SimpleObject};
use speedb::IteratorMode;
use zkapp::ZkappAccount;

#[derive(InputObject)]
pub struct AccountQueryInput {
    public_key: Option<String>,
    delegate: Option<String>,
    username: Option<String>,
    balance: Option<u64>,
    token: Option<String>,
    zkapp: Option<bool>,

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

#[derive(SimpleObject)]
pub struct Account {
    public_key: String,
    delegate: String,
    balance: u64,
    nonce: u32,
    time_locked: bool,
    timing: Option<Timing>,
    token: String,
    zkapp: Option<ZkappAccount>,
}

#[derive(Enum, Copy, Clone, Default, Eq, PartialEq)]
pub enum AccountSortByInput {
    BalanceAsc,

    #[default]
    BalanceDesc,
}

#[derive(SimpleObject)]
pub struct AccountWithMeta {
    #[graphql(flatten)]
    pub account: Account,

    #[graphql(name = "is_genesis_account")]
    is_genesis_account: bool,

    #[graphql(name = "pk_epoch_num_blocks")]
    pk_epoch_num_blocks: u32,

    #[graphql(name = "pk_total_num_blocks")]
    pk_total_num_blocks: u32,

    #[graphql(name = "pk_epoch_num_snarks")]
    pk_epoch_num_snarks: u32,

    #[graphql(name = "pk_total_num_snarks")]
    pk_total_num_snarks: u32,

    #[graphql(name = "pk_epoch_num_user_commands")]
    pk_epoch_num_user_commands: u32,

    #[graphql(name = "pk_total_num_user_commands")]
    pk_total_num_user_commands: u32,

    #[graphql(name = "pk_epoch_num_zkapp_commands")]
    pk_epoch_num_zkapp_commands: u32,

    #[graphql(name = "pk_total_num_zkapp_commands")]
    pk_total_num_zkapp_commands: u32,

    #[graphql(name = "pk_epoch_num_internal_commands")]
    pk_epoch_num_internal_commands: u32,

    #[graphql(name = "pk_total_num_internal_commands")]
    pk_total_num_internal_commands: u32,

    #[graphql(name = "block_height")]
    block_height: u32,

    username: Option<String>,
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
    ) -> Result<Vec<AccountWithMeta>> {
        use AccountSortByInput::*;

        let db = db(ctx);
        let sort_by = sort_by.unwrap_or_default();

        // query or default token
        let token = query
            .as_ref()
            .map_or(TokenAddress::default(), |q| match q.token.as_ref() {
                Some(token) => TokenAddress::new(token).expect("valid token address"),
                None => TokenAddress::default(),
            });

        // public key query handler
        if let Some(public_key) = query.as_ref().and_then(|q| q.public_key.as_ref()) {
            if let Ok(pk) = PublicKey::new(public_key) {
                return Ok(db
                    .get_best_account_display(&pk, &token)?
                    .iter()
                    .filter_map(|acct| {
                        let username = match db.get_username(&pk) {
                            Ok(None) | Err(_) => None,
                            Ok(Some(username)) => Some(username.0),
                        };

                        if query.as_ref().unwrap().matches(acct, username.as_ref()) {
                            let account = AccountWithMeta::new(db, acct.to_owned());
                            return Some(account);
                        }

                        None
                    })
                    .collect());
            } else {
                return Err(async_graphql::Error::new(format!(
                    "Invalid public key: {}",
                    public_key
                )));
            }
        }

        // token query handler
        if let Some(token) = query.as_ref().and_then(|q| q.token.as_ref()) {
            return query
                .as_ref()
                .unwrap()
                .token_query_handler(db, token as &str, sort_by, limit);
        }

        let mode = match sort_by {
            BalanceAsc => IteratorMode::Start,
            BalanceDesc => IteratorMode::End,
        };

        // default query handler use balance-sorted accounts
        let iter = match query.as_ref().and_then(|q| q.zkapp) {
            None | Some(false) => db.best_ledger_account_balance_iterator(mode).flatten(),
            Some(true) => db
                .zkapp_best_ledger_account_balance_iterator(mode)
                .flatten(),
        };
        let mut accounts = Vec::with_capacity(limit);

        for (_, value) in iter {
            if accounts.len() >= limit {
                break;
            }

            let account = serde_json::from_slice::<account::Account>(&value)?.display();
            let pk = account.public_key.clone();
            let username = match db.get_username(&pk) {
                Ok(None) | Err(_) => None,
                Ok(Some(username)) => Some(username.0),
            };

            if query
                .as_ref()
                .map_or(true, |q| q.matches(&account, username.as_ref()))
            {
                let account_with_meta = AccountWithMeta::new(db, account);
                accounts.push(account_with_meta);
            }
        }

        Ok(accounts)
    }
}

impl AccountQueryInput {
    fn matches(&self, account: &account::Account, username: Option<&String>) -> bool {
        let AccountQueryInput {
            public_key,
            delegate,
            username: query_username_prefix,
            balance,
            balance_gt,
            balance_gte,
            balance_lt,
            balance_lte,
            balance_ne,
            token,
            zkapp,
        } = self;

        if let Some(public_key) = public_key {
            if *public_key != account.public_key.0 {
                return false;
            }
        }

        if let Some(delegate) = delegate {
            if *delegate != account.delegate.0 {
                return false;
            }
        }

        if let Some(username_prefix) = query_username_prefix {
            if username.map_or(true, |u| {
                !u.to_lowercase()
                    .starts_with(&username_prefix.to_lowercase())
            }) {
                return false;
            }
        }

        if let Some(balance) = balance {
            if account.balance.0 != *balance {
                return false;
            }
        }

        if let Some(balance_gt) = balance_gt {
            if account.balance.0 <= *balance_gt {
                return false;
            }
        }

        if let Some(balance_gte) = balance_gte {
            if account.balance.0 < *balance_gte {
                return false;
            }
        }

        if let Some(balance_lt) = balance_lt {
            if account.balance.0 >= *balance_lt {
                return false;
            }
        }

        if let Some(balance_lte) = balance_lte {
            if account.balance.0 > *balance_lte {
                return false;
            }
        }

        if let Some(balance_ne) = balance_ne {
            if account.balance.0 == *balance_ne {
                return false;
            }
        }

        if let Some(token) = token.as_ref() {
            if account
                .token
                .as_ref()
                .map_or(token != MINA_TOKEN_ADDRESS, |t| {
                    *t != TokenAddress::new(token).expect("valid token address")
                })
            {
                return false;
            }
        }

        if let Some(zkapp) = zkapp {
            if account.is_zkapp_account() != *zkapp {
                return false;
            }
        }

        true
    }

    fn token_query_handler(
        &self,
        db: &std::sync::Arc<IndexerStore>,
        token: &str,
        sort_by: AccountSortByInput,
        limit: usize,
    ) -> Result<Vec<AccountWithMeta>> {
        // validate token
        if TokenAddress::new(token).is_none() {
            return Err(async_graphql::Error::new(format!(
                "Invalid token address: {}",
                token
            )));
        }

        // iterator mode
        let mut start = [0u8; TokenAddress::LEN + U64_LEN + 1];
        start[..TokenAddress::LEN].copy_from_slice(token.as_bytes());

        let mode = match sort_by {
            AccountSortByInput::BalanceAsc => {
                IteratorMode::From(&start, speedb::Direction::Forward)
            }
            AccountSortByInput::BalanceDesc => {
                // go beyond current token accounts
                start[TokenAddress::LEN..][..U64_LEN].copy_from_slice(&u64::MAX.to_be_bytes());
                start[TokenAddress::LEN..][U64_LEN..].copy_from_slice("Z".as_bytes());

                IteratorMode::From(&start, speedb::Direction::Reverse)
            }
        };

        // iterator
        let iter = match self.zkapp {
            None | Some(false) => db.best_ledger_account_balance_iterator(mode).flatten(),
            Some(true) => db
                .zkapp_best_ledger_account_balance_iterator(mode)
                .flatten(),
        };
        let mut accounts = Vec::with_capacity(limit);

        // iterate
        for (key, value) in iter {
            if key[..TokenAddress::LEN] != *token.as_bytes() || accounts.len() >= limit {
                // beyond desired token accounts or limit
                break;
            }

            let account = serde_json::from_slice::<account::Account>(&value)?.display();
            let pk = account.public_key.clone();
            let username = match db.get_username(&pk) {
                Ok(None) | Err(_) => None,
                Ok(Some(username)) => Some(username.0),
            };

            if self.matches(&account, username.as_ref()) {
                let account_with_meta = AccountWithMeta::new(db, account);
                accounts.push(account_with_meta);
            }
        }

        Ok(accounts)
    }
}

impl AccountWithMeta {
    pub fn new(db: &std::sync::Arc<IndexerStore>, account: account::Account) -> Self {
        let pk = account.public_key.to_owned();

        Self {
            is_genesis_account: account.genesis_account,
            pk_epoch_num_blocks: db
                .get_block_production_pk_epoch_count(&pk, None)
                .expect("pk epoch block count"),
            pk_total_num_blocks: db
                .get_block_production_pk_total_count(&pk)
                .expect("pk total block count"),
            pk_epoch_num_snarks: db
                .get_snarks_pk_epoch_count(&pk, None)
                .expect("pk epoch snark count"),
            pk_total_num_snarks: db
                .get_snarks_pk_total_count(&pk)
                .expect("pk total snark count"),
            pk_epoch_num_user_commands: db
                .get_user_commands_pk_epoch_count(&pk, None)
                .expect("pk epoch user command count"),
            pk_total_num_user_commands: db
                .get_user_commands_pk_total_count(&pk)
                .expect("pk total user command count"),
            pk_epoch_num_zkapp_commands: db
                .get_zkapp_commands_pk_epoch_count(&pk, None)
                .expect("pk epoch zkapp command count"),
            pk_total_num_zkapp_commands: db
                .get_zkapp_commands_pk_total_count(&pk)
                .expect("pk total zkapp command count"),
            pk_epoch_num_internal_commands: db
                .get_internal_commands_pk_epoch_count(&pk, None)
                .expect("pk epoch internal command count"),
            pk_total_num_internal_commands: db
                .get_internal_commands_pk_total_count(&pk)
                .expect("pk total internal command count"),
            block_height: db
                .get_best_block_height()
                .unwrap()
                .expect("best block height"),
            username: db
                .get_username(&pk)
                .expect("username")
                .map(|user| user.0)
                .or(Some("Unknown".to_string())),
            account: account.into(),
        }
    }
}

impl From<account::Account> for Account {
    fn from(value: account::Account) -> Self {
        Self {
            public_key: value.public_key.0,
            delegate: value.delegate.0,
            nonce: value.nonce.map_or(0, |n| n.0),
            balance: value.balance.0,
            time_locked: value.timing.is_some(),
            timing: value.timing.map(Into::into),
            token: value.token.map_or(MINA_TOKEN_ADDRESS.to_string(), |t| t.0),
            zkapp: value.zkapp.map(Into::into),
        }
    }
}

impl From<AccountWithMeta> for Account {
    fn from(value: AccountWithMeta) -> Self {
        Self {
            public_key: value.account.public_key,
            delegate: value.account.delegate,
            nonce: value.account.nonce,
            balance: value.account.balance,
            time_locked: value.account.timing.is_some(),
            timing: value.account.timing,
            token: value.account.token,
            zkapp: value.account.zkapp,
        }
    }
}

impl From<account::Timing> for Timing {
    fn from(timing: account::Timing) -> Self {
        Self {
            initial_minimum_balance: Some(timing.initial_minimum_balance.0),
            cliff_time: Some(timing.cliff_time.0),
            cliff_amount: Some(timing.cliff_amount.0),
            vesting_period: Some(timing.vesting_period.0),
            vesting_increment: Some(timing.vesting_increment.0),
        }
    }
}
