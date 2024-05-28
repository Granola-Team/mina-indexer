use super::db;
use crate::{
    block::store::BlockStore,
    command::store::CommandStore,
    ledger::{account, public_key::PublicKey, store::LedgerStore},
    store::account_balance_iterator,
    web::graphql::Timing,
};
use async_graphql::{Context, Enum, InputObject, Object, Result, SimpleObject};
use speedb::IteratorMode;

#[derive(SimpleObject)]
pub struct Account {
    public_key: String,
    username: Option<String>,
    delegate: String,
    balance: u64,
    nonce: u32,
    time_locked: bool,
    timing: Option<Timing>,

    #[graphql(name = "pk_epoch_num_blocks")]
    pk_epoch_num_blocks: u32,

    #[graphql(name = "pk_total_num_blocks")]
    pk_total_num_blocks: u32,

    #[graphql(name = "pk_epoch_num_user_commands")]
    pk_epoch_num_user_commands: u32,

    #[graphql(name = "pk_total_num_user_commands")]
    pk_total_num_user_commands: u32,
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
        let ledger = match db.get_ledger_state_hash(&state_hash, true) {
            Ok(Some(ledger)) => ledger,
            Ok(None) | Err(_) => {
                return Ok(None);
            }
        };

        // public key query handler
        if let Some(public_key) = query.as_ref().and_then(|q| q.public_key.clone()) {
            return Ok(ledger
                .accounts
                .get(&public_key.into())
                .filter(|acct| query.unwrap().matches(acct))
                .map(|acct| {
                    let pk = acct.public_key.clone();
                    vec![Account::from((
                        acct.clone(),
                        db.get_block_production_pk_epoch_count(&pk, None)
                            .expect("pk epoch block count"),
                        db.get_block_production_pk_total_count(&pk)
                            .expect("pk total block count"),
                        db.get_user_commands_pk_epoch_count(&pk, None)
                            .expect("pk epoch user commands count"),
                        db.get_user_commands_pk_total_count(&pk)
                            .expect("pk total user commands count"),
                    ))]
                }));
        }

        // TODO default query handler use balance-sorted accounts
        let mut accounts: Vec<Account> = Vec::with_capacity(limit);
        let best_ledger = db.get_best_ledger()?.expect("best ledger");
        let mode = match sort_by {
            Some(AccountSortByInput::BalanceAsc) => IteratorMode::Start,
            Some(AccountSortByInput::BalanceDesc) | None => IteratorMode::End,
        };

        for (key, _) in account_balance_iterator(db, mode).flatten() {
            let pk = PublicKey::from_bytes(&key[8..])?;
            let account = best_ledger.accounts.get(&pk).expect("account in ledger");

            if query.as_ref().map_or(true, |q| q.matches(account)) {
                let account = (
                    account.clone(),
                    db.get_block_production_pk_epoch_count(&pk, None)
                        .expect("pk epoch block count"),
                    db.get_block_production_pk_total_count(&pk)
                        .expect("pk total block count"),
                    db.get_user_commands_pk_epoch_count(&pk, None)
                        .expect("pk epoch user command count"),
                    db.get_user_commands_pk_total_count(&pk)
                        .expect("pk total user command count"),
                )
                    .into();
                accounts.push(account);

                if accounts.len() == limit {
                    break;
                }
            }
        }

        Ok(Some(accounts))
    }
}

impl AccountQueryInput {
    fn matches(&self, account: &account::Account) -> bool {
        let AccountQueryInput {
            public_key,
            // username,
            balance,
            balance_gt,
            balance_gte,
            balance_lt,
            balance_lte,
            balance_ne,
        } = self;
        if let Some(public_key) = public_key {
            return *public_key == account.public_key.0;
        }
        // TODO
        // if let Some(username) = username {
        //     return account
        //         .username
        //         .as_ref()
        //         .map_or(false, |u| *username == u.0);
        // }
        if let Some(balance) = balance {
            return *balance == account.balance.0;
        }
        if let Some(balance_gt) = balance_gt {
            return *balance_gt < account.balance.0;
        }
        if let Some(balance_gte) = balance_gte {
            return *balance_gte <= account.balance.0;
        }
        if let Some(balance_lt) = balance_lt {
            return *balance_lt > account.balance.0;
        }
        if let Some(balance_lte) = balance_lte {
            return *balance_lte >= account.balance.0;
        }
        if let Some(balance_ne) = balance_ne {
            return *balance_ne != account.balance.0;
        }
        true
    }
}

impl From<(account::Account, u32, u32, u32, u32)> for Account {
    fn from(account: (account::Account, u32, u32, u32, u32)) -> Self {
        Self {
            public_key: account.0.public_key.0,
            delegate: account.0.delegate.0,
            nonce: account.0.nonce.0,
            balance: account.0.balance.0,
            time_locked: account.0.timing.is_some(),
            timing: account.0.timing.map(|t| t.into()),
            username: account
                .0
                .username
                .map_or(Some("Unknown".to_string()), |u| Some(u.0)),
            pk_epoch_num_blocks: account.1,
            pk_total_num_blocks: account.2,
            pk_epoch_num_user_commands: account.3,
            pk_total_num_user_commands: account.4,
        }
    }
}

impl From<account::Timing> for Timing {
    fn from(timing: account::Timing) -> Self {
        Self {
            initial_minimum_balance: Some(timing.initial_minimum_balance),
            cliff_time: Some(timing.cliff_time),
            cliff_amount: Some(timing.cliff_amount),
            vesting_period: Some(timing.vesting_period),
            vesting_increment: Some(timing.vesting_increment),
        }
    }
}
