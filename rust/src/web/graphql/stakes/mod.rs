use super::db;
use crate::{
    block::store::BlockStore,
    chain_id::{chain_id, store::ChainIdStore},
    constants::*,
    ledger::{staking::StakingAccount, store::LedgerStore},
};
use async_graphql::{Context, Enum, InputObject, Object, Result, SimpleObject};
use rust_decimal::{prelude::ToPrimitive, Decimal};

#[derive(InputObject)]
pub struct StakesQueryInput {
    epoch: Option<u32>,
    delegate: Option<String>,
    ledger_hash: Option<String>,

    #[graphql(name = "public_key")]
    public_key: Option<String>,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum StakesSortByInput {
    BalanceDesc,
}

#[derive(Default)]
pub struct StakesQueryRoot;

#[Object]
impl StakesQueryRoot {
    // Cache for 1 day
    #[graphql(cache_control(max_age = 86400))]
    async fn stakes<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        query: Option<StakesQueryInput>,
        sort_by: Option<StakesSortByInput>,
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
        let mut accounts: Vec<StakesLedgerAccountWithMeta> = staking_ledger
            .staking_ledger
            .into_values()
            .filter(|account| {
                if let Some(ref query) = query {
                    let StakesQueryInput {
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

                let timing = account.timing.as_ref().map(|timing| StakesTiming {
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
                        total_delegated,
                        total_delegated_nanomina,
                        count_delegates,
                    },
                    timing,
                }
            })
            .collect();

        if let Some(StakesSortByInput::BalanceDesc) = sort_by {
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
    timing: Option<StakesTiming>,
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
    pub token: u32,
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
pub struct StakesDelegationTotals {
    /// Value total delegated
    pub total_delegated: f64,
    /// Value total delegated in nanomina
    pub total_delegated_nanomina: u64,
    /// Value count delegates
    pub count_delegates: u32,
}

#[derive(SimpleObject)]
pub struct StakesTiming {
    #[graphql(name = "cliff_amount")]
    pub cliff_amount: Option<u64>,
    #[graphql(name = "cliff_time")]
    pub cliff_time: Option<u64>,
    #[graphql(name = "initial_minimum_balance")]
    pub initial_minimum_balance: Option<u64>,
    #[graphql(name = "vesting_increment")]
    pub vesting_increment: Option<u64>,
    #[graphql(name = "vesting_period")]
    pub vesting_period: Option<u64>,
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
        let token = acc.token;
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
