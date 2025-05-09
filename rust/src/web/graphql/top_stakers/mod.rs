//! GraphQL `topStakers` endpoint

use super::{
    db,
    pk::{PK, PK_},
    stakes::StakesDelegationTotals,
};
use crate::{
    base::{amount::Amount, public_key::PublicKey, state_hash::StateHash},
    block::store::BlockStore,
    ledger::store::staking::StakingLedgerStore,
    utility::store::common::{from_be_bytes, U32_LEN},
};
use async_graphql::{Context, Enum, InputObject, Object, Result, SimpleObject};
use speedb::Direction;

#[derive(InputObject)]
pub struct TopStakersQueryInput {
    /// Input spoch
    epoch: u32,

    /// Input genesis state hash
    #[graphql(name = "genesis_state_hash")]
    genesis_state_hash: Option<String>,
}

#[derive(Default, Enum, Copy, Clone, Eq, PartialEq)]
pub enum TopStakersSortByInput {
    #[default]
    NumCanonicalBlocksProducedDesc,
    NumCanonicalBlocksProducedAsc,
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
                    "ledger hash unknown for epoch {} genesis {}",
                    epoch, genesis_state_hash
                )))
            }
        };
        let total_currency = db.get_total_currency(&ledger_hash)?.unwrap_or_default();

        let mut accounts = Vec::new();
        let direction = match sort_by.unwrap_or_default() {
            TopStakersSortByInput::NumCanonicalBlocksProducedAsc => Direction::Forward,
            TopStakersSortByInput::NumCanonicalBlocksProducedDesc => Direction::Reverse,
        };

        for (key, _) in db
            .canonical_epoch_blocks_produced_iterator(
                Some(&genesis_state_hash),
                Some(epoch),
                direction,
            )
            .flatten()
        {
            if key[..StateHash::LEN] != *genesis_state_hash.0.as_bytes()
                || key[StateHash::LEN..][..U32_LEN] != epoch.to_be_bytes()
                || accounts.len() >= limit
            {
                // gone beyond desired region or limit
                break;
            }

            let num = from_be_bytes(key[StateHash::LEN..][U32_LEN..][..U32_LEN].to_vec());
            let pk = PublicKey::from_bytes(&key[StateHash::LEN..][U32_LEN..][U32_LEN..])?;

            let delegations = db
                .get_epoch_delegations(&pk, epoch, &genesis_state_hash)?
                .expect("epoch delegations");

            let account = TopStakerAccount::new(
                db,
                db.get_block_production_pk_epoch_count(
                    &pk,
                    Some(epoch),
                    Some(&genesis_state_hash),
                )?,
                num,
                db.get_block_production_pk_supercharged_epoch_count(
                    &pk,
                    Some(epoch),
                    Some(&genesis_state_hash),
                )?,
                db.get_pk_epoch_slots_produced_count(&pk, Some(epoch), Some(&genesis_state_hash))?,
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

            accounts.push(account);
        }

        Ok(accounts)
    }
}

impl TopStakerAccount {
    fn new(
        db: &std::sync::Arc<crate::store::IndexerStore>,
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
