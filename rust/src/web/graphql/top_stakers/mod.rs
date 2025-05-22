//! GraphQL `topStakers` endpoint

use super::{
    db,
    pk::{PK, PK_},
    stakes::StakesDelegationTotals,
};
use crate::{
    base::{amount::Amount, public_key::PublicKey, state_hash::StateHash, username::Username},
    block::store::BlockStore,
    ledger::store::staking::StakingLedgerStore,
    store::IndexerStore,
    utility::store::common::{u32_from_be_bytes, U32_LEN},
};
use async_graphql::{Context, Enum, InputObject, Object, Result, SimpleObject};
use speedb::Direction;
use std::sync::Arc;

#[derive(InputObject)]
pub struct TopStakersQueryInput {
    /// Input epoch
    epoch: u32,

    /// Input genesis state hash
    #[graphql(name = "genesis_state_hash")]
    genesis_state_hash: Option<String>,

    /// Input staker public key
    #[graphql(name = "public_key")]
    public_key: Option<String>,

    /// Input staker username
    username: Option<String>,
}

#[derive(Default, Enum, Copy, Clone, Eq, PartialEq)]
pub enum TopStakersSortByInput {
    #[default]
    /// Sort by number of canonical blocks produced descending
    NumCanonicalBlocksProducedDesc,
    /// Sort by number of canonical blocks produced ascending
    NumCanonicalBlocksProducedAsc,

    /// Sort by number of slot produced descending
    NumSlotsProducedDesc,
    /// Sort by number of slot produced ascending
    NumSlotsProducedAsc,
}

#[derive(Default)]
pub struct TopStakersQueryRoot;

#[derive(SimpleObject)]
pub struct TopStakerAccount {
    /// Value staker public key
    #[graphql(name = "public_key", flatten)]
    public_key: PK_,

    /// Value epoch delegation totals
    #[graphql(name = "delegation_totals")]
    delegation_totals: StakesDelegationTotals,

    /// Value epoch blocks produced count
    #[graphql(name = "num_blocks_produced")]
    num_blocks_produced: u32,

    /// Value epoch canonical blocks produced count
    #[graphql(name = "num_canonical_blocks_produced")]
    num_canonical_blocks_produced: u32,

    /// Value epoch supercharged blocks count
    #[graphql(name = "num_supercharged_blocks_produced")]
    num_supercharged_blocks_produced: u32,

    /// Value epoch slots produced count
    #[graphql(name = "num_slots_produced")]
    num_slots_produced: u32,
}

#[Object]
impl TopStakersQueryRoot {
    async fn top_stakers(
        &self,
        ctx: &Context<'_>,
        query: Option<TopStakersQueryInput>,
        sort_by: Option<TopStakersSortByInput>,
        #[graphql(default = 100)] limit: usize,
    ) -> Result<Vec<TopStakerAccount>> {
        let db = db(ctx);
        let epoch = query.as_ref().map_or_else(
            || db.get_current_epoch().expect("current epoch"),
            |q| q.epoch,
        );

        let genesis_state_hash = query
            .as_ref()
            .and_then(|q| q.genesis_state_hash.clone())
            .or_else(|| {
                db.get_best_block_genesis_hash()
                    .expect("best block genesis state hash")
                    .map(|g| g.0)
            });
        let genesis_state_hash = match StateHash::new(genesis_state_hash.unwrap()) {
            Ok(genesis_state_hash) => genesis_state_hash,
            Err(e) => return Err(async_graphql::Error::from(e)),
        };

        let ledger_hash = db.get_staking_ledger_hash_by_epoch(epoch, &genesis_state_hash)?;
        let ledger_hash = match ledger_hash {
            Some(ledger_hash) => ledger_hash,
            None => {
                return Err(async_graphql::Error::new(format!(
                    "Ledger hash unknown for epoch {} genesis {}",
                    epoch, genesis_state_hash
                )))
            }
        };
        let total_currency = db.get_total_currency(&ledger_hash)?.unwrap_or_default();

        TopStakersQueryInput::verify_inputs(query.as_ref())?;
        TopStakersQueryInput::handler(
            db,
            query.as_ref(),
            epoch,
            &genesis_state_hash,
            sort_by.unwrap_or_default(),
            total_currency,
            limit,
        )
    }
}

impl TopStakersQueryInput {
    fn handler(
        db: &Arc<IndexerStore>,
        query: Option<&Self>,
        epoch: u32,
        genesis_state_hash: &StateHash,
        sort_by: TopStakersSortByInput,
        total_currency: u64,
        limit: usize,
    ) -> Result<Vec<TopStakerAccount>> {
        use TopStakersSortByInput::*;

        let direction = match sort_by {
            NumCanonicalBlocksProducedAsc | NumSlotsProducedAsc => Direction::Forward,
            NumCanonicalBlocksProducedDesc | NumSlotsProducedDesc => Direction::Reverse,
        };
        let iter = match sort_by {
            NumCanonicalBlocksProducedAsc | NumCanonicalBlocksProducedDesc => db
                .canonical_epoch_blocks_produced_iterator(
                    Some(epoch),
                    Some(genesis_state_hash),
                    direction,
                ),
            NumSlotsProducedAsc | NumSlotsProducedDesc => {
                db.epoch_slots_produced_iterator(Some(epoch), Some(genesis_state_hash), direction)
            }
        };

        let mut top_stakers = Vec::new();
        for (key, _) in iter.flatten() {
            if key[..StateHash::LEN] != *genesis_state_hash.0.as_bytes()
                || key[StateHash::LEN..][..U32_LEN] != epoch.to_be_bytes()
                || top_stakers.len() >= limit
            {
                // gone beyond desired region or limit
                break;
            }

            let pk = PublicKey::from_bytes(&key[StateHash::LEN..][U32_LEN..][U32_LEN..])?;
            let num_canonical_blocks_produced = match sort_by {
                NumCanonicalBlocksProducedAsc | NumCanonicalBlocksProducedDesc => {
                    u32_from_be_bytes(&key[StateHash::LEN..][U32_LEN..][..U32_LEN])?
                }
                _ => db.get_block_production_pk_canonical_epoch_count(
                    &pk,
                    Some(epoch),
                    Some(genesis_state_hash),
                )?,
            };
            let num_slots_produced = match sort_by {
                NumSlotsProducedAsc | NumSlotsProducedDesc => {
                    u32_from_be_bytes(&key[StateHash::LEN..][U32_LEN..][..U32_LEN])?
                }
                _ => db.get_pk_epoch_slots_produced_count(
                    &pk,
                    Some(epoch),
                    Some(genesis_state_hash),
                )?,
            };
            let delegations = db
                .get_epoch_delegations(&pk, epoch, genesis_state_hash)?
                .expect("epoch delegations");

            let top_staker = TopStakerAccount::new(
                db,
                db.get_block_production_pk_epoch_count(&pk, Some(epoch), Some(genesis_state_hash))?,
                num_canonical_blocks_produced,
                db.get_block_production_pk_supercharged_epoch_count(
                    &pk,
                    Some(epoch),
                    Some(genesis_state_hash),
                )?,
                num_slots_produced,
                pk,
                StakesDelegationTotals {
                    total_currency,
                    count_delegates: delegations.count_delegates,
                    total_delegated: Amount(delegations.total_delegated).to_f64(),
                    total_delegated_nanomina: delegations.total_delegated,
                    delegates: delegations
                        .delegates
                        .iter()
                        .map(|pk| pk.to_string())
                        .collect(),
                    delegate_pks: delegations
                        .delegates
                        .into_iter()
                        .map(|pk| PK::new(db, pk))
                        .collect(),
                },
            );

            if TopStakersQueryInput::matches(query, &top_staker) {
                top_stakers.push(top_staker);
            }
        }

        Ok(top_stakers)
    }

    fn verify_inputs(query: Option<&Self>) -> Result<()> {
        if let Some(public_key) = query.and_then(|q| q.public_key.as_ref()) {
            if !PublicKey::is_valid(public_key as &str) {
                return Err(async_graphql::Error::new(format!(
                    "Invalid public key: {}",
                    public_key
                )));
            }
        }

        if let Some(username) = query.and_then(|q| q.username.as_ref()) {
            if !Username::is_valid(username as &str) {
                return Err(async_graphql::Error::new(format!(
                    "Invalid username: {}",
                    username
                )));
            }
        }

        Ok(())
    }

    fn matches(query: Option<&Self>, top_staker: &TopStakerAccount) -> bool {
        if let Some(Self {
            epoch: _,
            genesis_state_hash: _,
            public_key,
            username,
        }) = query
        {
            if let Some(public_key) = public_key {
                if top_staker.public_key.public_key != *public_key {
                    return false;
                }
            }

            if let Some(username) = username {
                if let Some(staker_username) = top_staker.public_key.username.as_ref() {
                    if staker_username != username {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }

        true
    }
}

impl TopStakerAccount {
    fn new(
        db: &Arc<IndexerStore>,
        num_blocks_produced: u32,
        num_canonical_blocks_produced: u32,
        num_supercharged_blocks_produced: u32,
        num_slots_produced: u32,
        pk: PublicKey,
        delegation_totals: StakesDelegationTotals,
    ) -> Self {
        Self {
            public_key: PK_::new(db, pk),
            num_blocks_produced,
            num_canonical_blocks_produced,
            num_supercharged_blocks_produced,
            num_slots_produced,
            delegation_totals,
        }
    }
}
