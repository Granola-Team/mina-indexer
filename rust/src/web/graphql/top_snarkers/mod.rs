//! GraphQL `topSnarkers` endpoint

use super::{db, pk::PK_};
use crate::{
    base::{public_key::PublicKey, state_hash::StateHash},
    block::store::BlockStore,
    snark_work::store::SnarkStore,
    store::IndexerStore,
    utility::store::common::{u64_from_be_bytes, U32_LEN, U64_LEN},
};
use async_graphql::{Context, Enum, InputObject, Object, Result, SimpleObject};
use speedb::{DBIterator, Direction};
use std::sync::Arc;

#[derive(InputObject)]
pub struct TopSnarkersQueryInput {
    /// Input epoch
    epoch: u32,

    /// Input genesis state hash
    #[graphql(name = "genesis_state_hash")]
    genesis_state_hash: Option<String>,
}

#[derive(Enum, Copy, Clone, Default, Eq, PartialEq)]
pub enum TopSnarkersSortByInput {
    #[default]
    /// Sort by epoch total fees descending
    TotalFeesDesc,
    /// Sort by epoch total fees ascending
    TotalFeesAsc,

    /// Sort by epoch max fee descending
    MaxFeeDesc,
    /// Sort by epoch max fee ascending
    MaxFeeAsc,
}

#[derive(Default)]
pub struct TopSnarkersQueryRoot;

#[derive(SimpleObject)]
pub struct TopSnarker {
    /// Value SNARK prover public key
    #[graphql(flatten)]
    public_key: PK_,

    /// Value all-time total SNARK fees
    #[graphql(name = "total_fees")]
    total_fees: u64,

    /// Value epoch total SNARK fees
    #[graphql(name = "epoch_fees")]
    epoch_fees: u64,

    /// Value all-time min SNARK fee
    #[graphql(name = "min_fee")]
    min_fee: u64,

    /// Value epoch min SNARK fee
    #[graphql(name = "epoch_min_fee")]
    epoch_min_fee: u64,

    /// Value all-time max SNARK fee
    #[graphql(name = "max_fee")]
    max_fee: u64,

    /// Value epoch max SNARK fee
    #[graphql(name = "epoch_max_fee")]
    epoch_max_fee: u64,

    /// Value all-time SNARKs sold
    #[graphql(name = "snarks_sold")]
    snarks_sold: u32,

    /// Value epoch SNARKs sold
    #[graphql(name = "epoch_snarks_sold")]
    epoch_snarks_sold: u32,
}

#[Object]
impl TopSnarkersQueryRoot {
    async fn top_snarkers(
        &self,
        ctx: &Context<'_>,
        query: Option<TopSnarkersQueryInput>,
        sort_by: Option<TopSnarkersSortByInput>,
        #[graphql(default = 100)] limit: usize,
    ) -> Result<Vec<TopSnarker>> {
        use TopSnarkersSortByInput::*;

        let db = db(ctx);
        let epoch = query
            .as_ref()
            .map_or(db.get_current_epoch().expect("current epoch"), |q| q.epoch);

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

        let sort_by = sort_by.unwrap_or_default();
        let iter = match sort_by {
            TotalFeesAsc => db.snark_prover_total_fees_epoch_iterator(
                epoch,
                &genesis_state_hash,
                Direction::Forward,
            ),
            TotalFeesDesc => db.snark_prover_total_fees_epoch_iterator(
                epoch,
                &genesis_state_hash,
                Direction::Reverse,
            ),
            MaxFeeAsc => db.snark_prover_max_fee_epoch_iterator(
                epoch,
                &genesis_state_hash,
                Direction::Forward,
            ),
            MaxFeeDesc => db.snark_prover_max_fee_epoch_iterator(
                epoch,
                &genesis_state_hash,
                Direction::Reverse,
            ),
        };

        query_handler(db, iter, epoch, &genesis_state_hash, sort_by, limit)
    }
}

fn query_handler(
    db: &Arc<IndexerStore>,
    iter: DBIterator<'_>,
    epoch: u32,
    genesis_state_hash: &StateHash,
    sort_by: TopSnarkersSortByInput,
    limit: usize,
) -> Result<Vec<TopSnarker>> {
    use TopSnarkersSortByInput::*;
    let mut snarkers = vec![];

    for (key, _) in iter.flatten() {
        if key[..StateHash::LEN] != *genesis_state_hash.0.as_bytes()
            || key[StateHash::LEN..][..U32_LEN] != epoch.to_be_bytes()
            || snarkers.len() >= limit
        {
            // gone beyond the desired epoch or limit
            break;
        }

        let pk = PublicKey::from_bytes(&key[StateHash::LEN..][U32_LEN..][U64_LEN..])?;
        let fee = u64_from_be_bytes(&key[StateHash::LEN..][U32_LEN..][..U64_LEN])?;
        let epoch_fees = match sort_by {
            TotalFeesAsc | TotalFeesDesc => fee,
            MaxFeeAsc | MaxFeeDesc => db
                .get_snark_prover_epoch_fees(&pk, Some(epoch), Some(genesis_state_hash), None)?
                .expect("epoch fees"),
        };
        let epoch_max_fee = match sort_by {
            MaxFeeAsc | MaxFeeDesc => fee,
            TotalFeesAsc | TotalFeesDesc => db
                .get_snark_prover_epoch_max_fee(&pk, Some(epoch), Some(genesis_state_hash), None)?
                .expect("epoch max fee"),
        };

        snarkers.push(TopSnarker {
            epoch_fees,
            epoch_max_fee,
            epoch_min_fee: db
                .get_snark_prover_epoch_min_fee(&pk, Some(epoch), Some(genesis_state_hash), None)?
                .expect("epoch min fee"),
            epoch_snarks_sold: db.get_snarks_pk_epoch_count(
                &pk,
                Some(epoch),
                Some(genesis_state_hash),
            )?,
            total_fees: db
                .get_snark_prover_total_fees(&pk, None)?
                .expect("total fees"),
            min_fee: db.get_snark_prover_min_fee(&pk, None)?.expect("min fee"),
            max_fee: db.get_snark_prover_max_fee(&pk, None)?.expect("max fee"),
            snarks_sold: db.get_snarks_pk_total_count(&pk)?,
            public_key: PK_::new(db, pk),
        });
    }

    Ok(snarkers)
}
