//! GraphQL `stakes` endpoint

use super::db;
use crate::{
    base::amount::Amount,
    block::store::BlockStore,
    chain::store::ChainStore,
    command::{internal::store::InternalCommandStore, store::UserCommandStore},
    constants::MAINNET_GENESIS_HASH,
    ledger::{
        staking::{EpochStakeDelegation, StakingAccount},
        store::staking::{StakingAccountWithEpochDelegation, StakingLedgerStore},
    },
    snark_work::store::SnarkStore,
    store::{username::UsernameStore, IndexerStore},
    utility::store::common::U32_LEN,
    web::graphql::Timing,
};
use async_graphql::{ComplexObject, Context, Enum, InputObject, Object, Result, SimpleObject};
use rust_decimal::{prelude::ToPrimitive, Decimal};
use speedb::Direction;
use std::sync::Arc;

#[derive(InputObject, Default)]
pub struct StakeQueryInput {
    epoch: Option<u32>,
    delegate: Option<String>,
    ledger_hash: Option<String>,

    #[graphql(validator(regex = "^\\d+(\\.\\d{1,9})?$"), name = "stake_lte")]
    stake_lte: Option<String>,

    #[graphql(name = "public_key")]
    public_key: Option<String>,
    username: Option<String>,
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
    ) -> Result<Vec<StakesLedgerAccountWithMeta>> {
        let db = db(ctx);

        // default to current epoch
        let curr_epoch = db.get_current_epoch()?;
        let epoch = match query {
            Some(ref query) => query.epoch.unwrap_or(curr_epoch),
            None => curr_epoch,
        };

        // short-circuited epoch number query
        if limit == 0 {
            if let Some(ledger_hash) = query.as_ref().and_then(|q| q.ledger_hash.clone()) {
                return match db.get_epoch(&ledger_hash.clone().into())? {
                    Some(epoch) => Ok(vec![StakesLedgerAccountWithMeta {
                        epoch,
                        ledger_hash,
                        ..Default::default()
                    }]),
                    None => Ok(vec![]),
                };
            }
        }

        // if ledger hash is provided as a query input, use it for the ledger
        // otherwise, use the provided or current epoch number
        let (ledger_hash, epoch) = match query.as_ref().map(|q| (q.ledger_hash.clone(), q.epoch)) {
            Some((Some(ledger_hash), Some(query_epoch))) => (ledger_hash, query_epoch),
            Some((Some(ledger_hash), None)) => (
                ledger_hash.clone(),
                db.get_epoch(&ledger_hash.clone().into())?
                    .unwrap_or_default(),
            ),
            Some((None, Some(query_epoch))) => (
                db.get_staking_ledger_hash_by_epoch(query_epoch, None)?
                    .unwrap_or_default()
                    .0,
                query_epoch,
            ),
            Some((None, None)) | None => (
                db.get_staking_ledger_hash_by_epoch(epoch, None)?
                    .unwrap_or_default()
                    .0,
                epoch,
            ),
        };
        let total_currency = db
            .get_total_currency(&ledger_hash.clone().into())?
            .unwrap_or_default();

        // balance/stake-sorted queries
        let mut accounts = Vec::new();
        let iter = match sort_by {
            Some(StakeSortByInput::StakeDesc) | None => {
                db.staking_ledger_account_stake_iterator(epoch, Direction::Reverse)
            }
            Some(StakeSortByInput::StakeAsc) => {
                db.staking_ledger_account_stake_iterator(epoch, Direction::Forward)
            }
            Some(StakeSortByInput::BalanceDesc) => {
                db.staking_ledger_account_balance_iterator(epoch, Direction::Reverse)
            }
            Some(StakeSortByInput::BalanceAsc) => {
                db.staking_ledger_account_balance_iterator(epoch, Direction::Forward)
            }
        };

        for (key, value) in iter.flatten() {
            if key[..U32_LEN] != epoch.to_be_bytes() || accounts.len() >= limit {
                // no longer the desired staking ledger
                break;
            }

            let StakingAccountWithEpochDelegation {
                account,
                delegation,
            } = serde_json::from_slice(&value)?;
            if StakeQueryInput::matches_staking_account(
                query.as_ref(),
                &account,
                &ledger_hash,
                epoch,
            ) {
                let account = StakesLedgerAccountWithMeta::new(
                    db,
                    account,
                    &delegation,
                    epoch,
                    ledger_hash.clone(),
                    total_currency,
                );
                if StakeQueryInput::matches(query.as_ref(), &account) {
                    accounts.push(account);
                }
            }
        }
        Ok(accounts)
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

    /// Value epoch num supercharged blocks
    #[graphql(name = "epoch_num_supercharged_blocks")]
    epoch_num_supercharged_blocks: u32,

    /// Value total num blocks
    #[graphql(name = "total_num_blocks")]
    total_num_blocks: u32,

    /// Value total num supercharged blocks
    #[graphql(name = "total_num_supercharged_blocks")]
    total_num_supercharged_blocks: u32,

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

    /// Value total num accounts
    #[graphql(name = "epoch_num_accounts")]
    epoch_num_accounts: u32,
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

    /// Value public key
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

    /// Value pk epoch num supercharged blocks
    #[graphql(name = "pk_epoch_num_supercharged_blocks")]
    pub pk_epoch_num_supercharged_blocks: u32,

    /// Value pk total num supercharged blocks
    #[graphql(name = "pk_total_num_supercharged_blocks")]
    pub pk_total_num_supercharged_blocks: u32,

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

    /// Value delegates
    pub delegates: Vec<String>,
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

impl
    From<(
        StakingAccount,
        String,
        u32,
        u32,
        u32,
        u32,
        u32,
        u32,
        u32,
        u32,
        u32,
        u32,
        Option<String>,
    )> for StakesLedgerAccount
{
    fn from(
        acc: (
            StakingAccount,
            String,
            u32,
            u32,
            u32,
            u32,
            u32,
            u32,
            u32,
            u32,
            u32,
            u32,
            Option<String>,
        ),
    ) -> Self {
        let account = acc.0;

        let balance_nanomina = account.balance;
        let balance = Amount(balance_nanomina);
        let balance = balance.to_f64();

        let pk = account.pk.0;
        let public_key = pk.clone();
        let delegate = account.delegate.0;

        let nonce = account.nonce.unwrap_or_default().0;
        let token = account.token.unwrap_or_default();
        let receipt_chain_hash = account.receipt_chain_hash.0;
        let voting_for = account.voting_for.0;

        Self {
            chain_id: acc.1,
            balance,
            nonce,
            delegate,
            pk,
            public_key,
            token,
            receipt_chain_hash,
            voting_for,
            balance_nanomina,
            pk_epoch_num_blocks: acc.2,
            pk_total_num_blocks: acc.3,
            pk_epoch_num_supercharged_blocks: acc.4,
            pk_total_num_supercharged_blocks: acc.5,
            pk_epoch_num_snarks: acc.6,
            pk_total_num_snarks: acc.7,
            pk_epoch_num_user_commands: acc.8,
            pk_total_num_user_commands: acc.9,
            pk_epoch_num_internal_commands: acc.10,
            pk_total_num_internal_commands: acc.11,
            username: acc.12,
        }
    }
}

impl StakeQueryInput {
    pub fn matches(
        query: Option<&Self>,
        stakes_ledger_account: &StakesLedgerAccountWithMeta,
    ) -> bool {
        if let Some(query) = query {
            if let Some(stake_lte) = query.stake_lte.as_ref().and_then(|s| s.parse::<f64>().ok()) {
                if stakes_ledger_account.delegation_totals.total_delegated > stake_lte {
                    return false;
                }
            }
        }
        true
    }

    pub fn matches_staking_account(
        query: Option<&Self>,
        account: &StakingAccount,
        ledger_hash: &String,
        epoch: u32,
    ) -> bool {
        if let Some(query) = query {
            let Self {
                delegate,
                public_key,
                epoch: query_epoch,
                ledger_hash: query_ledger_hash,
                username,
                stake_lte: _,
            } = query;
            if let Some(public_key) = public_key {
                if *public_key != account.pk.0 {
                    return false;
                }
            }
            if let Some(username) = username {
                if let Some(acct_username) = account.username.as_ref() {
                    if *username != *acct_username {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            if let Some(delegate) = delegate {
                if *delegate != account.delegate.0 {
                    return false;
                }
            }
            if let Some(query_ledger_hash) = query_ledger_hash {
                if query_ledger_hash != ledger_hash {
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
    }
}

impl StakesLedgerAccountWithMeta {
    pub fn new(
        db: &Arc<IndexerStore>,
        account: StakingAccount,
        delegations: &EpochStakeDelegation,
        epoch: u32,
        ledger_hash: String,
        total_currency: u64,
    ) -> Self {
        let pk = account.pk.clone();
        let total_delegated_nanomina = delegations.total_delegated.unwrap_or_default();
        let count_delegates = delegations.count_delegates.unwrap_or_default();
        let delegates: Vec<String> = delegations
            .delegates
            .iter()
            .map(|pk| pk.0.clone())
            .collect();
        let mut decimal = Decimal::from(total_delegated_nanomina);
        decimal.set_scale(9).ok();

        let total_delegated = decimal.to_f64().unwrap_or_default();
        let timing = account.timing.as_ref().map(|timing| Timing {
            cliff_amount: Some(timing.cliff_amount.0),
            cliff_time: Some(timing.cliff_time.0),
            initial_minimum_balance: Some(timing.initial_minimum_balance.0),
            vesting_increment: Some(timing.vesting_increment.0),
            vesting_period: Some(timing.vesting_period.0),
        });
        let chain_id = db.get_chain_id().expect("chain id").0;

        // pk data counts
        let pk_epoch_num_blocks = db
            .get_block_production_pk_epoch_count(&pk, Some(epoch))
            .expect("pk epoch num blocks");
        let pk_total_num_blocks = db
            .get_block_production_pk_total_count(&pk)
            .expect("pk total num blocks");
        let pk_epoch_num_supercharged_blocks = db
            .get_block_production_pk_supercharged_epoch_count(&pk, Some(epoch))
            .expect("pk epoch num supercharged blocks");
        let pk_total_num_supercharged_blocks = db
            .get_block_production_pk_supercharged_total_count(&pk)
            .expect("pk total num supercharged blocks");
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
        let username = match db.get_username(&pk) {
            Ok(None) | Err(_) => Some("Unknown".to_string()),
            Ok(username) => username.map(|u| u.0),
        };

        Self {
            epoch,
            ledger_hash,
            account: StakesLedgerAccount::from((
                account,
                chain_id,
                pk_epoch_num_blocks,
                pk_total_num_blocks,
                pk_epoch_num_supercharged_blocks,
                pk_total_num_supercharged_blocks,
                pk_epoch_num_snarks,
                pk_total_num_snarks,
                pk_epoch_num_user_commands,
                pk_total_num_user_commands,
                pk_epoch_num_internal_commands,
                pk_total_num_internal_commands,
                username,
            )),
            delegation_totals: StakesDelegationTotals {
                count_delegates,
                total_delegated,
                total_delegated_nanomina,
                total_currency,
                delegates,
            },
            timing,
            epoch_num_blocks: db
                .get_block_production_epoch_count(Some(epoch))
                .expect("epoch block count"),
            epoch_num_supercharged_blocks: db
                .get_block_production_supercharged_epoch_count(Some(epoch))
                .expect("epoch supercharged block count"),
            total_num_blocks: db
                .get_block_production_total_count()
                .expect("total block count"),
            total_num_supercharged_blocks: db
                .get_block_production_supercharged_total_count()
                .expect("total supercharged block count"),
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
            epoch_num_accounts: db
                .get_staking_ledger_accounts_count_epoch(epoch, &MAINNET_GENESIS_HASH.into())
                .expect("total internal command count"),
        }
    }
}

#[cfg(test)]
mod web_graphql_stakes_tests {
    use super::*;

    #[test]
    fn test_matches_stake_lte_filter() {
        let stakes_ledger_account = StakesLedgerAccountWithMeta {
            delegation_totals: StakesDelegationTotals {
                total_delegated: 500_000.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let query_input_none = StakeQueryInput {
            stake_lte: None,
            ..Default::default()
        };
        assert!(StakeQueryInput::matches(
            Some(&query_input_none),
            &stakes_ledger_account
        ));

        let query_input_greater = StakeQueryInput {
            stake_lte: Some("600000.0".to_string()),
            ..Default::default()
        };
        assert!(StakeQueryInput::matches(
            Some(&query_input_greater),
            &stakes_ledger_account
        ));

        let query_input_equal = StakeQueryInput {
            stake_lte: Some("500000.0".to_string()),
            ..Default::default()
        };
        assert!(StakeQueryInput::matches(
            Some(&query_input_equal),
            &stakes_ledger_account
        ));

        let query_input_less = StakeQueryInput {
            stake_lte: Some("400000.0".to_string()),
            ..Default::default()
        };
        assert!(!StakeQueryInput::matches(
            Some(&query_input_less),
            &stakes_ledger_account
        ));
    }

    #[test]
    fn test_matches_no_filter() {
        let stakes_ledger_account = StakesLedgerAccountWithMeta {
            delegation_totals: StakesDelegationTotals {
                total_delegated: 800_000.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let query_input_none = StakeQueryInput::default();
        assert!(StakeQueryInput::matches(
            Some(&query_input_none),
            &stakes_ledger_account
        ));
    }
}
