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

                let timing = account.timing.as_ref().map(|timing| NextStakesTiming {
                    cliff_amount: Some(timing.cliff_amount),
                    cliff_time: Some(timing.cliff_time),
                    initial_minimum_balance: Some(timing.initial_minimum_balance),
                    vesting_increment: Some(timing.vesting_increment),
                    vesting_period: Some(timing.vesting_period),
                });

                NextStakesLedgerAccountWithMeta {
                    epoch,
                    ledger_hash: ledger_hash.clone(),
                    account: NextStakesLedgerAccount::from(account),
                    delegation_totals: NextStakesDelegationTotals {
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
    delegation_totals: NextStakesDelegationTotals,
    /// Value accounts
    #[graphql(flatten)]
    account: NextStakesLedgerAccount,
    /// Value timing
    timing: Option<NextStakesTiming>,
}

#[derive(SimpleObject)]
pub struct NextStakesLedgerAccount {
    /// Value chainId
    chain_id: String,
    /// Value balance
    balance: f64,
    /// Value nonce
    nonce: u32,
    /// Value delegate
    delegate: String,
    /// Value epoch
    pk: String,
    /// Value public key
    #[graphql(name = "public_key")]
    public_key: String,
    /// Value token
    token: u32,
    /// Value receipt chain hash
    #[graphql(name = "receipt_chain_hash")]
    receipt_chain_hash: String,
    /// Value voting for
    #[graphql(name = "voting_for")]
    voting_for: String,
    /// Value balance nanomina
    balance_nanomina: u64,
}

#[derive(SimpleObject)]
pub struct NextStakesDelegationTotals {
    /// Value total delegated
    total_delegated: f64,
    /// Value total delegated in nanomina
    total_delegated_nanomina: u64,
    /// Value count delegates
    count_delegates: u32,
}

#[derive(SimpleObject)]
struct NextStakesTiming {
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

impl From<StakingAccount> for NextStakesLedgerAccount {
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
