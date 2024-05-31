use super::db;
use crate::{
    block::store::BlockStore,
    chain::chain_id,
    command::{internal::store::InternalCommandStore, store::UserCommandStore},
    constants::*,
    ledger::{staking::StakingAccount, store::LedgerStore},
    snark_work::store::SnarkStore,
    web::graphql::Timing,
};
use async_graphql::{ComplexObject, Context, Enum, InputObject, Object, Result, SimpleObject};
use rust_decimal::{prelude::ToPrimitive, Decimal};

#[derive(InputObject)]
pub struct StakeQueryInput {
    epoch: Option<u32>,
    delegate: Option<String>,
    ledger_hash: Option<String>,

    #[graphql(name = "public_key")]
    public_key: Option<String>,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum StakeSortByInput {
    #[graphql(name = "BALANCE_ASC")]
    BalanceAsc,

    #[graphql(name = "BALANCE_DESC")]
    BalanceDesc,

    #[graphql(name = "STAKE_ASC")]
    StakeAsc,

    #[graphql(name = "STAKE_DESC")]
    StakeDesc,
}

#[derive(Default)]
pub struct StakeQueryRoot;

#[Object]
impl StakeQueryRoot {
    // Cache for 1 day
    #[graphql(cache_control(max_age = 86400))]
    async fn stakes<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        query: Option<StakeQueryInput>,
        sort_by: Option<StakeSortByInput>,
        #[graphql(default = 100)] limit: usize,
    ) -> Result<Option<Vec<StakesLedgerAccountWithMeta>>> {
        let db = db(ctx);

        // default to current epoch
        let curr_epoch = db.get_best_block()?.unwrap().epoch_count();
        let epoch = match query {
            Some(ref query) => query.epoch.unwrap_or(curr_epoch),
            None => curr_epoch,
        };

        // short-circuited epoch number query
        if limit == 0 {
            if let Some(ledger_hash) = query.as_ref().and_then(|q| q.ledger_hash.clone()) {
                return match db.get_epoch(&ledger_hash.clone().into())? {
                    Some(epoch) => Ok(Some(vec![StakesLedgerAccountWithMeta {
                        epoch,
                        ledger_hash,
                        ..Default::default()
                    }])),
                    None => Ok(None),
                };
            }
        }

        // if ledger hash is provided as a query input, use it
        // else, use the epoch number
        let staking_ledger = {
            let opt = if let Some(ledger_hash) = query.as_ref().and_then(|q| q.ledger_hash.clone())
            {
                db.get_staking_ledger_hash(&ledger_hash.into())?
            } else {
                db.get_staking_ledger_at_epoch(epoch, &None)?
            };
            match opt {
                Some(staking_ledger) => staking_ledger,
                None => return Ok(None),
            }
        };

        // Delegations will be present if the staking ledger is
        // (use the staking ledger's epoch)
        let epoch = staking_ledger.epoch;
        let delegations = db.get_delegations_epoch(epoch, &None)?.unwrap();

        let total_currency = staking_ledger.total_currency;
        let ledger_hash = staking_ledger.ledger_hash.clone().0;
        let mut accounts: Vec<StakesLedgerAccountWithMeta> = staking_ledger
            .staking_ledger
            .into_values()
            .filter(|account| {
                if let Some(ref query) = query {
                    let StakeQueryInput {
                        delegate,
                        public_key,
                        epoch: query_epoch,
                        ledger_hash: query_ledger_hash,
                    } = query;
                    if let Some(public_key) = public_key {
                        if *public_key != account.pk.0 {
                            return false;
                        }
                    }
                    if let Some(delegate) = delegate {
                        if *delegate != account.delegate.0 {
                            return false;
                        }
                    }
                    if let Some(query_ledger_hash) = query_ledger_hash {
                        if *query_ledger_hash != ledger_hash {
                            return false;
                        }
                    }
                    if let Some(query_epoch) = query_epoch {
                        if *query_epoch != epoch {
                            return false;
                        }
                    }
                }
                true
            })
            .map(|account| {
                let pk = account.pk.clone();
                let result = delegations
                    .delegations
                    .get(&pk)
                    .cloned()
                    .unwrap_or_default();
                let total_delegated_nanomina = result.total_delegated.unwrap_or_default();
                let count_delegates = result.count_delegates.unwrap_or_default();
                let mut decimal = Decimal::from(total_delegated_nanomina);
                decimal.set_scale(9).ok();

                let total_delegated = decimal.to_f64().unwrap_or_default();
                let timing = account.timing.as_ref().map(|timing| Timing {
                    cliff_amount: Some(timing.cliff_amount),
                    cliff_time: Some(timing.cliff_time),
                    initial_minimum_balance: Some(timing.initial_minimum_balance),
                    vesting_increment: Some(timing.vesting_increment),
                    vesting_period: Some(timing.vesting_period),
                });

                // pk data counts
                let pk_epoch_num_blocks = db
                    .get_block_production_pk_epoch_count(&pk, Some(epoch))
                    .expect("pk epoch num blocks");
                let pk_total_num_blocks = db
                    .get_block_production_pk_total_count(&pk)
                    .expect("pk total num blocks");
                let pk_epoch_num_snarks = db
                    .get_snarks_pk_epoch_count(&pk, Some(epoch))
                    .expect("pk epoch num snarks");
                let pk_total_num_snarks = db
                    .get_snarks_pk_total_count(&pk)
                    .expect("pk total num snarks");
                let pk_epoch_num_user_commands = db
                    .get_user_commands_pk_epoch_count(&pk, Some(epoch))
                    .expect("pk epoch num user commands");
                let pk_total_num_user_commands = db
                    .get_user_commands_pk_total_count(&pk)
                    .expect("pk total num user commands");
                let pk_epoch_num_internal_commands = db
                    .get_internal_commands_pk_epoch_count(&pk, Some(epoch))
                    .expect("pk epoch num internal commands");
                let pk_total_num_internal_commands = db
                    .get_internal_commands_pk_total_count(&pk)
                    .expect("pk total num internal commands");

                StakesLedgerAccountWithMeta {
                    epoch,
                    ledger_hash: ledger_hash.clone(),
                    account: StakesLedgerAccount::from((
                        account,
                        pk_epoch_num_blocks,
                        pk_total_num_blocks,
                        pk_epoch_num_snarks,
                        pk_total_num_snarks,
                        pk_epoch_num_user_commands,
                        pk_total_num_user_commands,
                        pk_epoch_num_internal_commands,
                        pk_total_num_internal_commands,
                    )),
                    delegation_totals: StakesDelegationTotals {
                        total_currency,
                        total_delegated,
                        total_delegated_nanomina,
                        count_delegates,
                    },
                    timing,
                    epoch_num_blocks: db
                        .get_block_production_epoch_count(epoch)
                        .expect("epoch block count"),
                    total_num_blocks: db
                        .get_block_production_total_count()
                        .expect("total block count"),
                    epoch_num_snarks: db
                        .get_snarks_epoch_count(Some(epoch))
                        .expect("epoch snark count"),
                    total_num_snarks: db.get_snarks_total_count().expect("total snark count"),
                    epoch_num_user_commands: db
                        .get_user_commands_epoch_count(Some(epoch))
                        .expect("epoch user command count"),
                    total_num_user_commands: db
                        .get_user_commands_total_count()
                        .expect("total user command count"),
                    epoch_num_internal_commands: db
                        .get_internal_commands_epoch_count(Some(epoch))
                        .expect("epoch internal command count"),
                    total_num_internal_commands: db
                        .get_internal_commands_total_count()
                        .expect("total internal command count"),
                }
            })
            .collect();

        match sort_by {
            Some(StakeSortByInput::BalanceAsc) => {
                accounts.sort_by(|a, b| a.account.balance_nanomina.cmp(&b.account.balance_nanomina))
            }
            Some(StakeSortByInput::BalanceDesc) => {
                accounts.sort_by(|a, b| b.account.balance_nanomina.cmp(&a.account.balance_nanomina))
            }
            Some(StakeSortByInput::StakeAsc) => {
                accounts.sort_by(|a, b| {
                    a.delegation_totals
                        .total_delegated_nanomina
                        .cmp(&b.delegation_totals.total_delegated_nanomina)
                });
            }
            Some(StakeSortByInput::StakeDesc) | None => {
                accounts.sort_by(|a, b| {
                    b.delegation_totals
                        .total_delegated_nanomina
                        .cmp(&a.delegation_totals.total_delegated_nanomina)
                });
            }
        }

        accounts.truncate(limit);
        Ok(Some(accounts))
    }
}

#[derive(SimpleObject, Default)]
pub struct StakesLedgerAccountWithMeta {
    /// Value current epoch
    epoch: u32,

    /// Value current ledger hash
    ledger_hash: String,

    /// Value delegation totals
    delegation_totals: StakesDelegationTotals,

    /// Value accounts
    #[graphql(flatten)]
    account: StakesLedgerAccount,

    /// Value timing
    timing: Option<Timing>,

    /// Value epoch num blocks
    #[graphql(name = "epoch_num_blocks")]
    epoch_num_blocks: u32,

    /// Value total num blocks
    #[graphql(name = "total_num_blocks")]
    total_num_blocks: u32,

    /// Value epoch num snarks
    #[graphql(name = "epoch_num_snarks")]
    epoch_num_snarks: u32,

    /// Value total num snarks
    #[graphql(name = "total_num_snarks")]
    total_num_snarks: u32,

    /// Value epoch num user commands
    #[graphql(name = "epoch_num_user_commands")]
    epoch_num_user_commands: u32,

    /// Value total num user commands
    #[graphql(name = "total_num_user_commands")]
    total_num_user_commands: u32,

    /// Value epoch num internal commands
    #[graphql(name = "epoch_num_internal_commands")]
    epoch_num_internal_commands: u32,

    /// Value total num internal commands
    #[graphql(name = "total_num_internal_commands")]
    total_num_internal_commands: u32,
}

#[derive(SimpleObject, Default)]
pub struct StakesLedgerAccount {
    /// Value chainId
    pub chain_id: String,

    /// Value balance
    pub balance: f64,

    /// Value nonce
    pub nonce: u32,

    /// Value delegate
    pub delegate: String,

    /// Value epoch
    pub pk: String,

    /// Value username
    pub username: Option<String>,

    /// Value public key
    #[graphql(name = "public_key")]
    pub public_key: String,

    /// Value token
    pub token: u64,

    /// Value receipt chain hash
    #[graphql(name = "receipt_chain_hash")]
    pub receipt_chain_hash: String,

    /// Value voting for
    #[graphql(name = "voting_for")]
    pub voting_for: String,

    /// Value balance nanomina
    pub balance_nanomina: u64,

    /// Value pk epoch num blocks
    #[graphql(name = "pk_epoch_num_blocks")]
    pub pk_epoch_num_blocks: u32,

    /// Value pk total num blocks
    #[graphql(name = "pk_total_num_blocks")]
    pub pk_total_num_blocks: u32,

    /// Value pk epoch num snarks
    #[graphql(name = "pk_epoch_num_snarks")]
    pk_epoch_num_snarks: u32,

    /// Value pk total num snarks
    #[graphql(name = "pk_total_num_snarks")]
    pk_total_num_snarks: u32,

    /// Value pk epoch num user commands
    #[graphql(name = "pk_epoch_num_user_commands")]
    pk_epoch_num_user_commands: u32,

    /// Value pk total num user commands
    #[graphql(name = "pk_total_num_user_commands")]
    pk_total_num_user_commands: u32,

    /// Value pk epoch num internal commands
    #[graphql(name = "pk_epoch_num_internal_commands")]
    pk_epoch_num_internal_commands: u32,

    /// Value pk total num internal commands
    #[graphql(name = "pk_total_num_internal_commands")]
    pk_total_num_internal_commands: u32,
}

#[derive(SimpleObject, Default)]
#[graphql(complex)]
pub struct StakesDelegationTotals {
    /// Value total currency
    pub total_currency: u64,

    /// Value total delegated
    pub total_delegated: f64,

    /// Value total delegated in nanomina
    pub total_delegated_nanomina: u64,

    /// Value count delegates
    pub count_delegates: u32,
}

#[ComplexObject]
impl StakesDelegationTotals {
    /// Value total stake percentage
    async fn total_stake_percentage(&self) -> String {
        let total_currency_decimal = Decimal::from(self.total_currency);
        let total_delegated_decimal = Decimal::from(self.total_delegated_nanomina);
        let ratio = if !total_currency_decimal.is_zero() {
            (total_delegated_decimal / total_currency_decimal) * Decimal::from(100)
        } else {
            Decimal::ZERO
        };
        let rounded_ratio = ratio.round_dp(2);
        format!("{:.2}", rounded_ratio)
    }
}

impl From<(StakingAccount, u32, u32, u32, u32, u32, u32, u32, u32)> for StakesLedgerAccount {
    fn from(acc: (StakingAccount, u32, u32, u32, u32, u32, u32, u32, u32)) -> Self {
        let balance_nanomina = acc.0.balance;
        let mut decimal = Decimal::from(balance_nanomina);
        decimal.set_scale(9).ok();

        let balance = decimal.to_f64().unwrap_or_default();
        let nonce = acc.0.nonce.unwrap_or_default();
        let delegate = acc.0.delegate.0;
        let pk = acc.0.pk.0;
        let public_key = pk.clone();
        let token = acc.0.token.unwrap_or_default();
        let receipt_chain_hash = acc.0.receipt_chain_hash.0;
        let voting_for = acc.0.voting_for.0;
        Self {
            chain_id: chain_id(
                MAINNET_GENESIS_HASH,
                MAINNET_PROTOCOL_CONSTANTS,
                MAINNET_CONSTRAINT_SYSTEM_DIGESTS,
            )
            .0[..6]
                .to_string(),
            balance,
            nonce,
            delegate,
            pk,
            public_key,
            token,
            receipt_chain_hash,
            voting_for,
            balance_nanomina,
            username: None,
            pk_epoch_num_blocks: acc.1,
            pk_total_num_blocks: acc.2,
            pk_epoch_num_snarks: acc.3,
            pk_total_num_snarks: acc.4,
            pk_epoch_num_user_commands: acc.5,
            pk_total_num_user_commands: acc.6,
            pk_epoch_num_internal_commands: acc.7,
            pk_total_num_internal_commands: acc.8,
        }
    }
}
