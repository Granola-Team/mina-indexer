use super::db;
use crate::{
    block::store::BlockStore,
    chain_id::store::ChainIdStore,
    constants::*,
    ledger::store::LedgerStore,
    web::graphql::stakes::{StakesDelegationTotals, StakesLedgerAccount, StakesTiming},
};
use async_graphql::{Context, Enum, InputObject, Object, Result, SimpleObject};
use rust_decimal::{prelude::ToPrimitive, Decimal};

#[derive(InputObject)]
pub struct NextStakesQueryInput {
    epoch: Option<u32>,
    delegate: Option<String>,
    ledger_hash: Option<String>,

    #[graphql(name = "public_key")]
    public_key: Option<String>,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum NextStakesSortByInput {
    #[graphql(name = "BALANCE_ASC")]
    BalanceAsc,
    #[graphql(name = "BALANCE_DESC")]
    BalanceDesc,
}

#[derive(Default)]
pub struct NextStakesQueryRoot;

#[Object]
impl NextStakesQueryRoot {
    // Cache for 1 day
    #[graphql(cache_control(max_age = 86400))]
    async fn next_stakes<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        query: Option<NextStakesQueryInput>,
        sort_by: Option<NextStakesSortByInput>,
        #[graphql(default = 100)] limit: usize,
    ) -> Result<Option<Vec<NextStakesLedgerAccountWithMeta>>> {
        let db = db(ctx);

        // default to next epoch
        let next_epoch = 1 + db.get_best_block()?.map_or(0, |block| {
            block.global_slot_since_genesis() / MAINNET_EPOCH_SLOT_COUNT
        });
        let epoch = match query {
            Some(ref query) => query.epoch.map_or(next_epoch, |e| e + 1),
            None => next_epoch,
        };
        let network = db
            .get_current_network()
            .map(|n| n.0)
            .unwrap_or("mainnet".to_string());
        let staking_ledger = match db.get_staking_ledger_at_epoch(&network, epoch)? {
            Some(staking_ledger) => staking_ledger,
            None => return Ok(None),
        };

        // Delegations will be present if the staking ledger is
        let delegations = db.get_delegations_epoch(&network, epoch)?.unwrap();

        let ledger_hash = staking_ledger.ledger_hash.clone().0;
        let mut accounts: Vec<NextStakesLedgerAccountWithMeta> = staking_ledger
            .staking_ledger
            .into_values()
            .filter(|account| {
                if let Some(ref query) = query {
                    let NextStakesQueryInput {
                        delegate,
                        public_key,
                        epoch: query_epoch,
                        ledger_hash: query_ledger_hash,
                    } = query;
                    if let Some(public_key) = public_key {
                        return *public_key == account.pk.0;
                    }
                    if let Some(delegate) = &query.delegate {
                        return *delegate == account.delegate.0;
                    }
                    if let Some(query_ledger_hash) = query_ledger_hash {
                        return *query_ledger_hash == ledger_hash;
                    }
                    if let Some(query_epoch) = query_epoch {
                        return *query_epoch == epoch;
                    }
                    if let Some(delegate) = delegate {
                        return *delegate == account.delegate.0;
                    }
                }
                true
            })
            .map(|account| {
                let pk = account.pk.clone();
                let result = delegations.delegations.get(&pk).unwrap();
                let total_delegated_nanomina = result.total_delegated.unwrap_or_default();
                let count_delegates = result.count_delegates.unwrap_or_default();
                let mut decimal = Decimal::from(total_delegated_nanomina);
                decimal.set_scale(9).ok();

                let total_delegated = decimal.to_f64().unwrap_or_default();

                let timing = account.timing.as_ref().map(|timing| StakesTiming {
                    cliff_amount: Some(timing.cliff_amount),
                    cliff_time: Some(timing.cliff_time),
                    initial_minimum_balance: Some(timing.initial_minimum_balance),
                    vesting_increment: Some(timing.vesting_increment),
                    vesting_period: Some(timing.vesting_period),
                });

                NextStakesLedgerAccountWithMeta {
                    epoch,
                    ledger_hash: ledger_hash.clone(),
                    account: StakesLedgerAccount::from(account),
                    next_delegation_totals: StakesDelegationTotals {
                        total_delegated,
                        total_delegated_nanomina,
                        count_delegates,
                    },
                    timing,
                }
            })
            .collect();

        match sort_by {
            Some(NextStakesSortByInput::BalanceAsc) => {
                accounts.sort_by(|b, a| b.account.balance_nanomina.cmp(&a.account.balance_nanomina))
            }
            Some(NextStakesSortByInput::BalanceDesc) => {
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
    timing: Option<StakesTiming>,
}
