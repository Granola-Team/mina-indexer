//! GraphQL `accounts` endpoint

mod zkapp;

use super::{
    db,
    pk::{DelegatePK, PK},
};
use crate::{
    base::public_key::PublicKey,
    block::store::BlockStore,
    command::{internal::store::InternalCommandStore, store::UserCommandStore},
    constants::MINA_TOKEN_ADDRESS,
    ledger::{
        account::{self, Permission},
        store::best::BestLedgerStore,
        token::TokenAddress,
    },
    snark_work::store::SnarkStore,
    store::{username::UsernameStore, IndexerStore},
    utility::store::common::U64_LEN,
    web::graphql::timing::Timing,
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
    /// Value public key
    #[graphql(flatten)]
    public_key: PK,

    /// Value delegate public key
    #[graphql(flatten)]
    delegate: DelegatePK,

    /// Value balance (nano)
    balance: u64,

    /// Value nonce
    nonce: u32,

    /// Value time locked
    time_locked: bool,

    /// Value account timing
    timing: Option<Timing>,

    /// Value account token address
    token: String,

    /// Value zkapp
    zkapp: Option<ZkappAccount>,

    /// Value receipt chain hash
    receipt_chain_hash: String,

    /// Value voting for
    voting_for: String,

    /// Value permissions
    permissions: Option<Permissions>,
}

#[derive(SimpleObject, Default, Debug, Clone, PartialEq, Eq)]
struct Permissions {
    #[graphql(name = "edit_state")]
    edit_state: String,

    #[graphql(name = "access")]
    access: String,

    #[graphql(name = "send")]
    send: String,

    #[graphql(name = "receive")]
    receive: String,

    #[graphql(name = "set_delegate")]
    set_delegate: String,

    #[graphql(name = "set_permissions")]
    set_permissions: String,

    #[graphql(name = "set_verification_key")]
    set_verification_key: PermissionVk,

    #[graphql(name = "set_zkapp_uri")]
    set_zkapp_uri: String,

    #[graphql(name = "edit_action_state")]
    edit_action_state: String,

    #[graphql(name = "set_token_symbol")]
    set_token_symbol: String,

    #[graphql(name = "increment_nonce")]
    increment_nonce: String,

    #[graphql(name = "set_voting_for")]
    set_voting_for: String,

    #[graphql(name = "set_timing")]
    set_timing: String,
}

#[derive(SimpleObject, Default, Debug, Clone, PartialEq, Eq)]
struct PermissionVk {
    permission: String,
    number: String,
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

    #[graphql(name = "genesis_account")]
    genesis_account: Option<u64>,

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

    // TODO deprecate
    username: String,
}

#[derive(Default)]
pub struct AccountQueryRoot;

#[Object]
impl AccountQueryRoot {
    #[graphql(cache_control(max_age = 3600))]
    async fn accounts(
        &self,
        ctx: &Context<'_>,
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
                    .into_iter()
                    .filter_map(|acct| {
                        let username = match db.get_username(&pk) {
                            Ok(None) | Err(_) => None,
                            Ok(Some(username)) => Some(username.0),
                        };

                        if query.as_ref().unwrap().matches(&acct, username.as_ref()) {
                            let account = AccountWithMeta::new(db, acct);
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

            let account = serde_json::from_slice::<account::Account>(&value)?
                .deduct_mina_account_creation_fee();
            let username = match db.get_username(&account.public_key) {
                Ok(None) | Err(_) => None,
                Ok(Some(username)) => Some(username.0),
            };

            if query
                .as_ref()
                .is_none_or(|q| q.matches(&account, username.as_ref()))
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
            if username.is_none_or(|u| {
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

            let account = serde_json::from_slice::<account::Account>(&value)?
                .deduct_mina_account_creation_fee();

            let pk = &account.public_key;
            let username = match db.get_username(pk) {
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
    /// Account creation fee must already be deducted
    pub fn new(db: &std::sync::Arc<IndexerStore>, account: account::Account) -> Self {
        let pk = &account.public_key;

        Self {
            is_genesis_account: account.genesis_account.is_some(),
            genesis_account: account.genesis_account.map(|amt| amt.0),
            pk_epoch_num_blocks: db
                .get_block_production_pk_epoch_count(pk, None, None)
                .expect("pk epoch block count"),
            pk_total_num_blocks: db
                .get_block_production_pk_total_count(pk)
                .expect("pk total block count"),
            pk_epoch_num_snarks: db
                .get_snarks_pk_epoch_count(pk, None, None)
                .expect("pk epoch snark count"),
            pk_total_num_snarks: db
                .get_snarks_pk_total_count(pk)
                .expect("pk total snark count"),
            pk_epoch_num_user_commands: db
                .get_user_commands_pk_epoch_count(pk, None, None)
                .expect("pk epoch user command count"),
            pk_total_num_user_commands: db
                .get_user_commands_pk_total_count(pk)
                .expect("pk total user command count"),
            pk_epoch_num_zkapp_commands: db
                .get_zkapp_commands_pk_epoch_count(pk, None, None)
                .expect("pk epoch zkapp command count"),
            pk_total_num_zkapp_commands: db
                .get_zkapp_commands_pk_total_count(pk)
                .expect("pk total zkapp command count"),
            pk_epoch_num_internal_commands: db
                .get_internal_commands_pk_epoch_count(pk, None, None)
                .expect("pk epoch internal command count"),
            pk_total_num_internal_commands: db
                .get_internal_commands_pk_total_count(pk)
                .expect("pk total internal command count"),
            block_height: db
                .get_best_block_height()
                .unwrap()
                .expect("best block height"),
            username: db.get_username(pk).expect("username").unwrap_or_default().0,
            account: Account::new(db, account),
        }
    }
}

impl Account {
    /// Creates a GQL account from a ledger account
    fn new(db: &std::sync::Arc<IndexerStore>, account: account::Account) -> Self {
        let permissions = if account.is_zkapp_account() {
            account.permissions.map(Into::into)
        } else {
            None
        };

        Self {
            public_key: PK::new(db, account.public_key),
            delegate: DelegatePK::new(db, account.delegate),
            nonce: account.nonce.map_or(0, |n| n.0),
            balance: account.balance.0,
            time_locked: account.timing.is_some(),
            timing: account.timing.map(Into::into),
            token: account
                .token
                .map_or(MINA_TOKEN_ADDRESS.to_string(), |t| t.0),
            zkapp: account.zkapp.map(Into::into),
            receipt_chain_hash: account.receipt_chain_hash.unwrap_or_default().0,
            voting_for: account.voting_for.unwrap_or_default().0,
            permissions,
        }
    }
}

/////////////////
// conversions //
/////////////////

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

impl From<account::Permissions> for Permissions {
    fn from(value: account::Permissions) -> Self {
        Self {
            edit_state: value.edit_state.to_string(),
            access: value.access.to_string(),
            send: value.send.to_string(),
            receive: value.receive.to_string(),
            set_delegate: value.set_delegate.to_string(),
            set_permissions: value.set_permissions.to_string(),
            set_verification_key: value.set_verification_key.into(),
            set_zkapp_uri: value.set_zkapp_uri.to_string(),
            edit_action_state: value.edit_action_state.to_string(),
            set_token_symbol: value.set_token_symbol.to_string(),
            increment_nonce: value.increment_nonce.to_string(),
            set_voting_for: value.set_voting_for.to_string(),
            set_timing: value.set_timing.to_string(),
        }
    }
}

impl From<(Permission, String)> for PermissionVk {
    fn from(value: (Permission, String)) -> Self {
        Self {
            permission: value.0.to_string(),
            number: value.1,
        }
    }
}
