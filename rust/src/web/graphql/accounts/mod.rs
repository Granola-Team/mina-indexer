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
    username: Option<String>,
    delegate: String,
    balance: u64,
    nonce: u32,
    time_locked: bool,
    timing: Option<Timing>,
    token: String,
    zkapp: Option<ZkappAccount>,

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
    ) -> Result<Vec<Account>> {
        use AccountSortByInput::*;

        let db = db(ctx);
        let token = query
            .as_ref()
            .map_or(TokenAddress::default(), |q| match q.token.as_ref() {
                Some(token) => TokenAddress::new(token).expect("valid token address"),
                None => TokenAddress::default(),
            });

        // public key query handler
        if let Some(public_key) = query.as_ref().and_then(|q| q.public_key.clone()) {
            let pk: PublicKey = public_key.into();
            return Ok(db
                .get_best_account_display(&pk, &token)?
                .iter()
                .filter_map(|acct| {
                    let username = match db.get_username(&pk) {
                        Ok(None) | Err(_) => None,
                        Ok(Some(username)) => Some(username.0),
                    };

                    if query.as_ref().unwrap().matches(acct, username.as_ref()) {
                        let account = Account::with_meta(db, acct.to_owned());

                        return Some(account);
                    }

                    None
                })
                .collect());
        }

        // default query handler use balance-sorted accounts
        let mode = match sort_by {
            Some(BalanceAsc) => IteratorMode::Start,
            Some(BalanceDesc) | None => IteratorMode::End,
        };
        let iter = match query.as_ref().and_then(|q| q.zkapp) {
            // all account types
            None | Some(false) => db.best_ledger_account_balance_iterator(mode).flatten(),
            // zkapp accounts only
            Some(true) => db
                .zkapp_best_ledger_account_balance_iterator(mode)
                .flatten(),
            // non-zkapp account only
            // Some(false) => todo!("non-zkapp account"),
        };
        let mut accounts = Vec::with_capacity(limit);

        for (_, value) in iter {
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
                let account_with_meta = AccountWithMeta {
                    account,
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
                    username,
                };
                accounts.push(account_with_meta.into());

                if accounts.len() >= limit {
                    break;
                }
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
            if account.token != Some(TokenAddress::new(token).expect("valid token address")) {
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
}

pub struct AccountWithMeta {
    pub account: account::Account,
    pub pk_epoch_num_blocks: u32,
    pub pk_total_num_blocks: u32,
    pub pk_epoch_num_snarks: u32,
    pub pk_total_num_snarks: u32,
    pub pk_epoch_num_user_commands: u32,
    pub pk_total_num_user_commands: u32,
    pub pk_epoch_num_zkapp_commands: u32,
    pub pk_total_num_zkapp_commands: u32,
    pub pk_epoch_num_internal_commands: u32,
    pub pk_total_num_internal_commands: u32,
    pub username: Option<String>,
}

impl Account {
    pub fn with_meta(db: &std::sync::Arc<IndexerStore>, account: account::Account) -> Self {
        let pk = &account.public_key;
        AccountWithMeta {
            username: account.username.as_ref().map(ToString::to_string),
            pk_epoch_num_blocks: db
                .get_block_production_pk_epoch_count(pk, None)
                .expect("pk epoch block count"),
            pk_total_num_blocks: db
                .get_block_production_pk_total_count(pk)
                .expect("pk total block count"),
            pk_epoch_num_snarks: db
                .get_snarks_pk_epoch_count(pk, None)
                .expect("pk epoch snark count"),
            pk_total_num_snarks: db
                .get_snarks_pk_total_count(pk)
                .expect("pk total snark count"),
            pk_epoch_num_user_commands: db
                .get_user_commands_pk_epoch_count(pk, None)
                .expect("pk epoch user command count"),
            pk_total_num_user_commands: db
                .get_user_commands_pk_total_count(pk)
                .expect("pk total user command count"),
            pk_epoch_num_zkapp_commands: db
                .get_zkapp_commands_pk_epoch_count(pk, None)
                .expect("pk epoch zkapp command count"),
            pk_total_num_zkapp_commands: db
                .get_zkapp_commands_pk_total_count(pk)
                .expect("pk total zkapp command count"),
            pk_epoch_num_internal_commands: db
                .get_internal_commands_pk_epoch_count(pk, None)
                .expect("pk epoch internal command count"),
            pk_total_num_internal_commands: db
                .get_internal_commands_pk_total_count(pk)
                .expect("pk total internal command count"),
            account,
        }
        .into()
    }
}

impl From<AccountWithMeta> for Account {
    fn from(account: AccountWithMeta) -> Self {
        Self {
            public_key: account.account.public_key.0,
            delegate: account.account.delegate.0,
            nonce: account.account.nonce.map_or(0, |n| n.0),
            balance: account.account.balance.0,
            time_locked: account.account.timing.is_some(),
            timing: account.account.timing.map(Into::into),
            is_genesis_account: account.account.genesis_account,
            token: account
                .account
                .token
                .map_or(MINA_TOKEN_ADDRESS.to_string(), |t| t.0),
            zkapp: account.account.zkapp.map(Into::into),
            pk_epoch_num_blocks: account.pk_epoch_num_blocks,
            pk_total_num_blocks: account.pk_total_num_blocks,
            pk_epoch_num_snarks: account.pk_epoch_num_snarks,
            pk_total_num_snarks: account.pk_total_num_snarks,
            pk_epoch_num_user_commands: account.pk_epoch_num_user_commands,
            pk_total_num_user_commands: account.pk_total_num_user_commands,
            pk_epoch_num_zkapp_commands: account.pk_epoch_num_zkapp_commands,
            pk_total_num_zkapp_commands: account.pk_total_num_zkapp_commands,
            pk_epoch_num_internal_commands: account.pk_epoch_num_internal_commands,
            pk_total_num_internal_commands: account.pk_total_num_internal_commands,
            username: account.username.or(Some("Unknown".to_string())),
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
