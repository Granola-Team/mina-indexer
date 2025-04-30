//! GraphQL `stakes` endpoint

use super::db;
use crate::{
    base::{amount::Amount, state_hash::StateHash},
    block::store::BlockStore,
    command::{internal::store::InternalCommandStore, store::UserCommandStore},
    constants::MINA_TOKEN_ID,
    ledger::{
        staking::{EpochStakeDelegation, StakingAccount, StakingLedger},
        store::staking::{StakingAccountWithEpochDelegation, StakingLedgerStore},
        LedgerHash,
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
pub struct StakesQueryInput {
    epoch: Option<u32>,
    delegate: Option<String>,
    ledger_hash: Option<String>,
    genesis_state_hash: Option<String>,

    #[graphql(validator(regex = "^\\d+(\\.\\d{1,9})?$"), name = "stake_lte")]
    stake_lte: Option<String>,

    #[graphql(name = "public_key")]
    public_key: Option<String>,
    username: Option<String>,
}

#[derive(Default, Enum, Copy, Clone, Eq, PartialEq)]
pub enum StakesSortByInput {
    #[graphql(name = "BALANCE_DESC")]
    BalanceDesc,
    #[graphql(name = "BALANCE_ASC")]
    BalanceAsc,

    #[default]
    #[graphql(name = "STAKE_DESC")]
    StakeDesc,
    #[graphql(name = "STAKE_ASC")]
    StakeAsc,
}

#[derive(SimpleObject, Default)]
pub struct StakesLedgerAccountWithMeta {
    /// Value current epoch
    epoch: u32,

    /// Value current ledger hash
    ledger_hash: String,

    /// Value genesis state hash
    genesis_state_hash: String,

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

    /// Value epoch num accounts
    #[graphql(name = "epoch_num_accounts")]
    epoch_num_accounts: u32,

    /// Value num accounts
    #[graphql(name = "num_accounts")]
    num_accounts: u32,
}

#[derive(SimpleObject, Default)]
pub struct StakesLedgerAccount {
    /// Value balance
    pub balance: f64,

    /// Value nonce
    pub nonce: u32,

    /// Value delegate
    pub delegate: String,

    /// Value public key
    pub pk: String,

    /// Value username
    pub username: String,

    /// Value public key
    #[graphql(name = "public_key")]
    pub public_key: String,

    /// Value token id
    pub token: u64,

    /// Value token address
    pub token_address: String,

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

#[derive(Default)]
pub struct StakesQueryRoot;

#[Object]
impl StakesQueryRoot {
    // Cache for 1 day
    #[graphql(cache_control(max_age = 86400))]
    async fn stakes(
        &self,
        ctx: &Context<'_>,
        query: Option<StakesQueryInput>,
        sort_by: Option<StakesSortByInput>,
        #[graphql(default = 100)] limit: usize,
    ) -> Result<Vec<StakesLedgerAccountWithMeta>> {
        let db = db(ctx);

        // default to current epoch
        let epoch = query
            .as_ref()
            .and_then(|q| q.epoch)
            .unwrap_or_else(|| db.get_current_epoch().expect("epoch"));

        // short-circuited epoch number query
        if limit == 0 {
            if let Some(ledger_hash) = query.as_ref().and_then(|q| q.ledger_hash.to_owned()) {
                return match db.get_epoch(&ledger_hash.to_owned().into())? {
                    Some(epoch) => Ok(vec![StakesLedgerAccountWithMeta {
                        epoch,
                        ledger_hash,
                        ..Default::default()
                    }]),
                    None => Ok(vec![]),
                };
            }
        }

        // default to best block genesis state hash
        let genesis_state_hash = match query.as_ref().and_then(|q| q.genesis_state_hash.as_ref()) {
            Some(genesis) => match StateHash::new(genesis) {
                Ok(genesis) => genesis,
                Err(_) => {
                    return Err(async_graphql::Error::new(format!(
                        "Invalid genesis state hash: {}",
                        genesis
                    )))
                }
            },
            None => db
                .get_best_block_genesis_hash()
                .ok()
                .flatten()
                .expect("genesis state hash"),
        };

        // if ledger hash is provided as a query input, use it for the ledger
        // otherwise, use the provided or current epoch number
        let (ledger_hash, epoch) = match query.as_ref().map(|q| (q.ledger_hash.as_ref(), q.epoch)) {
            Some((Some(ledger_hash), query_epoch)) => {
                let ledger_hash = match LedgerHash::new(ledger_hash) {
                    Ok(ledger_hash) => ledger_hash,
                    Err(_) => {
                        return Err(async_graphql::Error::new(format!(
                            "Invalid ledger hash: {}",
                            ledger_hash
                        )))
                    }
                };

                let epoch = if let Some(epoch) = query_epoch {
                    epoch
                } else {
                    db.get_epoch(&ledger_hash)?.unwrap_or_default()
                };

                (ledger_hash, epoch)
            }
            _ => (
                if let Some(ledger_hash) =
                    db.get_staking_ledger_hash_by_epoch(epoch, Some(&genesis_state_hash))?
                {
                    ledger_hash
                } else {
                    return Ok(vec![]);
                },
                epoch,
            ),
        };
        let total_currency = db.get_total_currency(&ledger_hash)?.unwrap_or_default();

        use StakesSortByInput::*;
        let mut accounts = Vec::with_capacity(limit);

        // username query
        if let Some(username) = query.as_ref().and_then(|q| q.username.as_ref()) {
            let username_pks = db.get_username_pks(username)?.unwrap_or_default();

            for pk in username_pks {
                // check limit
                if accounts.len() >= limit {
                    break;
                }

                if let Some(mut account) =
                    db.get_staking_account(&pk, epoch, Some(&genesis_state_hash))?
                {
                    if let Some(delegation) =
                        db.get_epoch_delegations(&pk, epoch, Some(&genesis_state_hash))?
                    {
                        // add username to account
                        account.username = db.get_username(&pk)?.map(|u| u.0);

                        if StakesQueryInput::matches_staking_account(
                            query.as_ref(),
                            &account,
                            &ledger_hash,
                            &genesis_state_hash,
                            epoch,
                        ) {
                            let account = StakesLedgerAccountWithMeta::new(
                                db,
                                account,
                                &delegation,
                                epoch,
                                ledger_hash.to_owned(),
                                total_currency,
                            );

                            if StakesQueryInput::matches(query.as_ref(), &account) {
                                accounts.push(account);
                            }
                        }
                    } else {
                        return Err(async_graphql::Error::new(format!(
                            "No staking delegations found for pk {} epoch {} genesis {}",
                            pk, epoch, genesis_state_hash,
                        )));
                    }
                } else {
                    return Err(async_graphql::Error::new(format!(
                        "No staking account found for pk {} epoch {} genesis {}",
                        pk, epoch, genesis_state_hash,
                    )));
                }
            }

            return Ok(accounts);
        }

        // balance/stake-sorted queries
        let iter = match sort_by.unwrap_or_default() {
            StakeDesc => db.staking_ledger_account_stake_iterator(
                epoch,
                &genesis_state_hash,
                Direction::Reverse,
            ),
            StakeAsc => db.staking_ledger_account_stake_iterator(
                epoch,
                &genesis_state_hash,
                Direction::Forward,
            ),
            BalanceDesc => db.staking_ledger_account_balance_iterator(
                epoch,
                &genesis_state_hash,
                Direction::Reverse,
            ),
            BalanceAsc => db.staking_ledger_account_balance_iterator(
                epoch,
                &genesis_state_hash,
                Direction::Forward,
            ),
        };

        for (key, value) in iter.flatten() {
            if key[..StateHash::LEN] != *genesis_state_hash.0.as_bytes()
                || key[StateHash::LEN..][..U32_LEN] != epoch.to_be_bytes()
                || accounts.len() >= limit
            {
                // no longer the desired staking ledger
                break;
            }

            let StakingAccountWithEpochDelegation {
                account,
                delegation,
            } = serde_json::from_slice(&value)?;

            if StakesQueryInput::matches_staking_account(
                query.as_ref(),
                &account,
                &ledger_hash,
                &genesis_state_hash,
                epoch,
            ) {
                let account = StakesLedgerAccountWithMeta::new(
                    db,
                    account,
                    &delegation,
                    epoch,
                    ledger_hash.to_owned(),
                    total_currency,
                );

                if StakesQueryInput::matches(query.as_ref(), &account) {
                    accounts.push(account);
                }
            }
        }

        Ok(accounts)
    }
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

        ratio.round_dp(2).to_string()
    }
}

impl
    From<(
        StakingAccount, //  0 - staking account
        u32,            //  1 - pk epoch num blocks
        u32,            //  2 - pk total num blocks
        u32,            //  3 - pk epoch num supercharged blocks
        u32,            //  4 - pk total num supercharged blocks
        u32,            //  5 - pk epoch num SNARKs
        u32,            //  6 - pk total num SNARKs
        u32,            //  7 - pk epoch num user commands
        u32,            //  8 - pk total num user commands
        u32,            //  9 - pk epoch num internal commands
        u32,            // 10 - pk total num internal commands
        String,         // 11 - username
    )> for StakesLedgerAccount
{
    fn from(
        acc: (
            StakingAccount, //  0 - staking account
            u32,            //  1 - pk epoch num blocks
            u32,            //  2 - pk total num blocks
            u32,            //  3 - pk epoch num supercharged blocks
            u32,            //  4 - pk total num supercharged blocks
            u32,            //  5 - pk epoch num SNARKs
            u32,            //  6 - pk total num SNARKs
            u32,            //  7 - pk epoch num user commands
            u32,            //  8 - pk total num user commands
            u32,            //  9 - pk epoch num internal commands
            u32,            // 10 - pk total num internal commands
            String,         // 11 - username
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
        let token = account.token.unwrap_or_default().0;
        let receipt_chain_hash = account.receipt_chain_hash.0;
        let voting_for = account.voting_for.0;

        Self {
            balance,
            nonce,
            delegate,
            pk,
            public_key,
            token: MINA_TOKEN_ID,
            token_address: token,
            receipt_chain_hash,
            voting_for,
            balance_nanomina,
            pk_epoch_num_blocks: acc.1,
            pk_total_num_blocks: acc.2,
            pk_epoch_num_supercharged_blocks: acc.3,
            pk_total_num_supercharged_blocks: acc.4,
            pk_epoch_num_snarks: acc.5,
            pk_total_num_snarks: acc.6,
            pk_epoch_num_user_commands: acc.7,
            pk_total_num_user_commands: acc.8,
            pk_epoch_num_internal_commands: acc.9,
            pk_total_num_internal_commands: acc.10,
            username: acc.11,
        }
    }
}

impl StakesQueryInput {
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
        ledger_hash: &LedgerHash,
        genesis_state_hash: &StateHash,
        epoch: u32,
    ) -> bool {
        if let Some(query) = query {
            let Self {
                delegate,
                public_key,
                epoch: query_epoch,
                ledger_hash: query_ledger_hash,
                genesis_state_hash: query_genesis_state_hash,
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
                }

                return false;
            }

            if let Some(delegate) = delegate {
                if *delegate != account.delegate.0 {
                    return false;
                }
            }

            if let Some(query_ledger_hash) = query_ledger_hash {
                if *query_ledger_hash != ledger_hash.0 {
                    return false;
                }
            }

            if let Some(query_genesis_state_hash) = query_genesis_state_hash {
                if *query_genesis_state_hash != genesis_state_hash.0 {
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
        ledger_hash: LedgerHash,
        total_currency: u64,
    ) -> Self {
        let pk = &account.pk;
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
        let genesis_state_hash = StakingLedger::genesis_state_hash(&ledger_hash);

        // pk data counts
        let pk_epoch_num_blocks = db
            .get_block_production_pk_epoch_count(pk, Some(epoch), Some(genesis_state_hash.clone()))
            .expect("pk epoch num blocks");
        let pk_total_num_blocks = db
            .get_block_production_pk_total_count(pk)
            .expect("pk total num blocks");
        let pk_epoch_num_supercharged_blocks = db
            .get_block_production_pk_supercharged_epoch_count(
                pk,
                Some(epoch),
                Some(genesis_state_hash),
            )
            .expect("pk epoch num supercharged blocks");
        let pk_total_num_supercharged_blocks = db
            .get_block_production_pk_supercharged_total_count(pk)
            .expect("pk total num supercharged blocks");
        let pk_epoch_num_snarks = db
            .get_snarks_pk_epoch_count(pk, Some(epoch))
            .expect("pk epoch num snarks");
        let pk_total_num_snarks = db
            .get_snarks_pk_total_count(pk)
            .expect("pk total num snarks");
        let pk_epoch_num_user_commands = db
            .get_user_commands_pk_epoch_count(pk, Some(epoch))
            .expect("pk epoch num user commands");
        let pk_total_num_user_commands = db
            .get_user_commands_pk_total_count(pk)
            .expect("pk total num user commands");
        let pk_epoch_num_internal_commands = db
            .get_internal_commands_pk_epoch_count(pk, Some(epoch))
            .expect("pk epoch num internal commands");
        let pk_total_num_internal_commands = db
            .get_internal_commands_pk_total_count(pk)
            .expect("pk total num internal commands");
        let username = db.get_username(pk).expect("username").unwrap_or_default().0;

        let genesis_state_hash = StakingLedger::genesis_state_hash(&ledger_hash);
        let num_accounts = db
            .get_staking_ledger_accounts_count_epoch(epoch, &genesis_state_hash)
            .expect("epoch staking account count");

        Self {
            epoch,
            ledger_hash: ledger_hash.0,
            genesis_state_hash: genesis_state_hash.to_string(),
            account: StakesLedgerAccount::from((
                account,
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
                .get_block_production_epoch_count(Some(genesis_state_hash.clone()), Some(epoch))
                .expect("epoch block count"),
            epoch_num_supercharged_blocks: db
                .get_block_production_supercharged_epoch_count(
                    Some(genesis_state_hash),
                    Some(epoch),
                )
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
            epoch_num_accounts: num_accounts,
            num_accounts,
        }
    }
}

#[cfg(all(test, feature = "tier2"))]
mod tests {
    use super::{StakesDelegationTotals, StakesLedgerAccountWithMeta, StakesQueryInput};
    use crate::{
        base::{public_key::PublicKey, state_hash::StateHash},
        chain::Network,
        ledger::{
            hash::LedgerHash,
            staking::{StakingAccount, StakingLedger},
            store::staking::StakingLedgerStore,
            username::Username,
        },
        store::{username::UsernameStore, IndexerStore},
    };
    use quickcheck::{Arbitrary, Gen};
    use std::collections::HashMap;
    use tempfile::TempDir;

    #[test]
    fn test_matches_stake_lte_filter() {
        let stakes_ledger_account = StakesLedgerAccountWithMeta {
            delegation_totals: StakesDelegationTotals {
                total_delegated: 500_000.0,
                ..Default::default()
            },
            ..Default::default()
        };

        let query_input_none = StakesQueryInput {
            stake_lte: None,
            ..Default::default()
        };
        assert!(StakesQueryInput::matches(
            Some(&query_input_none),
            &stakes_ledger_account
        ));

        let query_input_greater = StakesQueryInput {
            stake_lte: Some("600000.0".to_string()),
            ..Default::default()
        };
        assert!(StakesQueryInput::matches(
            Some(&query_input_greater),
            &stakes_ledger_account
        ));

        let query_input_equal = StakesQueryInput {
            stake_lte: Some("500000.0".to_string()),
            ..Default::default()
        };
        assert!(StakesQueryInput::matches(
            Some(&query_input_equal),
            &stakes_ledger_account
        ));

        let query_input_less = StakesQueryInput {
            stake_lte: Some("400000.0".to_string()),
            ..Default::default()
        };
        assert!(!StakesQueryInput::matches(
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

        let query_input_none = StakesQueryInput::default();
        assert!(StakesQueryInput::matches(
            Some(&query_input_none),
            &stakes_ledger_account
        ));
    }

    const GEN_SIZE: usize = 1000;
    fn create_indexer_store() -> anyhow::Result<IndexerStore> {
        let temp_dir = TempDir::with_prefix(std::env::current_dir()?)?;
        IndexerStore::new(temp_dir.path())
    }

    #[test]
    fn query_username() -> anyhow::Result<()> {
        let g = &mut Gen::new(GEN_SIZE);
        let store = std::sync::Arc::new(create_indexer_store()?);

        let pk = PublicKey::arbitrary(g);
        let account = StakingAccount {
            pk: pk.clone(),
            ..Default::default()
        };

        let epoch = u32::arbitrary(g);
        let ledger_hash = LedgerHash::arbitrary(g);
        let genesis_state_hash = StateHash::arbitrary(g);
        let total_currency = u64::arbitrary(g);

        let staking_ledger = StakingLedger {
            epoch,
            network: Network::arbitrary(g),
            ledger_hash: ledger_hash.clone(),
            total_currency,
            genesis_state_hash: genesis_state_hash.clone(),
            staking_ledger: HashMap::from([(pk.clone(), account)]),
        };

        // add staking ledger
        store.add_staking_ledger(staking_ledger, &genesis_state_hash)?;

        // add username
        let username = Username::arbitrary(g);
        store.add_username(pk.clone(), &username)?;

        // query the stakes endpoint by username
        let query = || {
            let mut accounts = vec![];
            let username_pks = store.get_username_pks(&username.0)?.unwrap_or_default();

            for pk in username_pks {
                if let Some(account) =
                    store.get_staking_account(&pk, epoch, Some(&genesis_state_hash))?
                {
                    if let Some(delegation) =
                        store.get_epoch_delegations(&pk, epoch, Some(&genesis_state_hash))?
                    {
                        let account = StakesLedgerAccountWithMeta::new(
                            &store,
                            account,
                            &delegation,
                            epoch,
                            ledger_hash.to_owned(),
                            total_currency,
                        );

                        accounts.push(account);
                    } else {
                        return Err(async_graphql::Error::new(format!(
                            "No staking delegations found for pk {} epoch {} genesis {}",
                            pk, epoch, genesis_state_hash,
                        )));
                    }
                } else {
                    return Err(async_graphql::Error::new(format!(
                        "No staking account found for pk {} epoch {} genesis {}",
                        pk, epoch, genesis_state_hash,
                    )));
                }
            }

            Ok(accounts)
        };

        let res = query()
            .unwrap()
            .iter()
            .map(|account| {
                (
                    account.account.public_key.clone(),
                    account.account.username.clone(),
                )
            })
            .collect::<Vec<_>>();

        assert_eq!(res, vec![(pk.to_string(), username.to_string())]);
        Ok(())
    }
}
