use super::db;
use crate::{
    block::store::BlockStore,
    chain::chain_id,
    constants::*,
    ledger::{staking::StakingAccount, store::LedgerStore},
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
    BalanceDesc,
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
        let curr_epoch = db.get_best_block()?.map_or(0, |block| {
            block.global_slot_since_genesis() / MAINNET_EPOCH_SLOT_COUNT
        });
        let epoch = match query {
            Some(ref query) => query.epoch.unwrap_or(curr_epoch),
            None => curr_epoch,
        };
        let staking_ledger = match db.get_staking_ledger_at_epoch(epoch, &None)? {
            Some(staking_ledger) => staking_ledger,
            None => return Ok(None),
        };
        // Delegations will be present if the staking ledger is
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
                        return *public_key == account.pk.0;
                    }
                    if let Some(delegate) = delegate {
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

                let timing = account.timing.as_ref().map(|timing| Timing {
                    cliff_amount: Some(timing.cliff_amount),
                    cliff_time: Some(timing.cliff_time),
                    initial_minimum_balance: Some(timing.initial_minimum_balance),
                    vesting_increment: Some(timing.vesting_increment),
                    vesting_period: Some(timing.vesting_period),
                });

                StakesLedgerAccountWithMeta {
                    epoch,
                    ledger_hash: ledger_hash.clone(),
                    account: StakesLedgerAccount::from(account),
                    delegation_totals: StakesDelegationTotals {
                        total_currency,
                        total_delegated,
                        total_delegated_nanomina,
                        count_delegates,
                    },
                    timing,
                }
            })
            .collect();

        if let Some(StakeSortByInput::BalanceDesc) = sort_by {
            accounts.sort_by(|a, b| b.account.balance_nanomina.cmp(&a.account.balance_nanomina));
        }

        accounts.truncate(limit);
        Ok(Some(accounts))
    }
}

#[derive(SimpleObject)]
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
}

#[derive(SimpleObject)]
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
}

#[derive(SimpleObject)]
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
        format!("{:.2}%", rounded_ratio)
    }
}

impl From<StakingAccount> for StakesLedgerAccount {
    fn from(acc: StakingAccount) -> Self {
        let balance_nanomina = acc.balance;
        let mut decimal = Decimal::from(balance_nanomina);
        decimal.set_scale(9).ok();

        let balance = decimal.to_f64().unwrap_or_default();
        let nonce = acc.nonce.unwrap_or_default();
        let delegate = acc.delegate.0;
        let pk = acc.pk.0;
        let public_key = pk.clone();
        let token = acc.token.unwrap_or_default();
        let receipt_chain_hash = acc.receipt_chain_hash.0;
        let voting_for = acc.voting_for.0;
        Self {
            chain_id: chain_id(
                MAINNET_GENESIS_HASH,
                MAINNET_GENESIS_CONSTANTS,
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
        }
    }
}
