use super::db;
use crate::{
    block::store::BlockStore,
    command::{internal::store::InternalCommandStore, store::UserCommandStore},
    ledger::store::LedgerStore,
    snark_work::store::SnarkStore,
    web::graphql::{
        stakes::{StakesDelegationTotals, StakesLedgerAccount},
        Timing,
    },
};
use async_graphql::{Context, Enum, InputObject, Object, Result, SimpleObject};
use rust_decimal::{prelude::ToPrimitive, Decimal};

#[derive(InputObject)]
pub struct NextstakeQueryInput {
    epoch: Option<u32>,
    delegate: Option<String>,
    ledger_hash: Option<String>,

    #[graphql(name = "public_key")]
    public_key: Option<String>,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum NextstakeSortByInput {
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
pub struct NextstakeQueryRoot;

#[Object]
impl NextstakeQueryRoot {
    // Cache for 1 day
    #[graphql(cache_control(max_age = 86400))]
    async fn nextstakes<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        query: Option<NextstakeQueryInput>,
        sort_by: Option<NextstakeSortByInput>,
        #[graphql(default = 100)] limit: usize,
    ) -> Result<Option<Vec<NextStakesLedgerAccountWithMeta>>> {
        let db = db(ctx);

        // default to next epoch
        let next_epoch = 1 + db.get_best_block()?.unwrap().epoch_count();
        let epoch = match query {
            Some(ref query) => query.epoch.map_or(next_epoch, |e| e + 1),
            None => next_epoch,
        };

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
        let epoch = staking_ledger.epoch;
        let total_currency = staking_ledger.total_currency;
        let ledger_hash = staking_ledger.ledger_hash.clone().0;
        let delegations = db
            .get_delegations_epoch(epoch, &None)?
            .expect("delegations are present if staking ledger is");

        // collect the results
        let mut accounts: Vec<NextStakesLedgerAccountWithMeta> = staking_ledger
            .staking_ledger
            .into_values()
            .filter(|account| {
                if let Some(ref query) = query {
                    let NextstakeQueryInput {
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
                        if *query_epoch + 1 != epoch {
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

                // nothing has been produced in the next epoch
                let pk_epoch_num_blocks = 0;
                let pk_epoch_num_snarks = 0;
                let pk_epoch_num_user_commands = 0;
                let pk_epoch_num_internal_commands = 0;

                let pk_total_num_blocks = db
                    .get_block_production_pk_total_count(&pk)
                    .expect("pk total num blocks");
                let pk_total_num_snarks = db
                    .get_snarks_pk_total_count(&pk)
                    .expect("pk total num snarks");
                let pk_total_num_user_commands = db
                    .get_user_commands_pk_total_count(&pk)
                    .expect("pk total num user commands");
                let pk_total_num_internal_commands = db
                    .get_internal_commands_pk_total_count(&pk)
                    .expect("pk total num internal commands");

                NextStakesLedgerAccountWithMeta {
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
                    next_delegation_totals: StakesDelegationTotals {
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
            Some(NextstakeSortByInput::BalanceAsc) | Some(NextstakeSortByInput::StakeAsc) => {
                accounts.sort_by(|b, a| b.account.balance_nanomina.cmp(&a.account.balance_nanomina))
            }
            Some(NextstakeSortByInput::BalanceDesc) | Some(NextstakeSortByInput::StakeDesc) => {
                accounts.sort_by(|a, b| b.account.balance_nanomina.cmp(&a.account.balance_nanomina))
            }
            None => (),
        }

        accounts.truncate(limit);
        Ok(Some(accounts))
    }
}

#[derive(SimpleObject)]
pub struct NextStakesLedgerAccountWithMeta {
    /// Value next epoch
    epoch: u32,

    /// Value next ledger hash
    ledger_hash: String,

    /// Value delegation totals
    next_delegation_totals: StakesDelegationTotals,

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
