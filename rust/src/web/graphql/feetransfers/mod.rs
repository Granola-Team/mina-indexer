//! GraphQL `feetransfers` endpoint

use super::{
    blocks::block::{Block, BlockWithoutCanonicity},
    gen::BlockQueryInput,
    get_block, get_block_canonicity,
};
use crate::{
    base::{public_key::PublicKey, state_hash::StateHash},
    block::{precomputed::PrecomputedBlock, store::BlockStore},
    command::{
        internal::{store::InternalCommandStore, DbInternalCommandWithData},
        store::UserCommandStore,
    },
    constants::*,
    snark_work::store::SnarkStore,
    store::IndexerStore,
    utility::store::common::{from_be_bytes, U32_LEN},
    web::graphql::db,
};
use async_graphql::{Context, Enum, InputObject, Object, Result, SimpleObject};
use speedb::{Direction, IteratorMode};
use std::sync::Arc;

#[derive(SimpleObject, Debug)]
pub struct Feetransfer {
    pub state_hash: String,
    pub fee: u64,
    pub recipient: String,
    pub block_height: u32,
    pub date_time: String,

    #[graphql(name = "type")]
    pub feetransfer_kind: String,

    #[graphql(name = "epoch_num_internal_commands")]
    epoch_num_internal_commands: u32,

    #[graphql(name = "total_num_internal_commands")]
    total_num_internal_commands: u32,
}

#[derive(Debug)]
pub struct FeetransferWithMeta {
    /// Value canonicity
    pub canonical: bool,

    /// Value optional block
    pub block: Option<PrecomputedBlock>,

    /// Value feetranser
    pub feetransfer: Feetransfer,
}

#[Object]
impl FeetransferWithMeta {
    async fn canonical(&self) -> bool {
        self.canonical
    }

    #[graphql(flatten)]
    async fn feetransfer(&self) -> &Feetransfer {
        &self.feetransfer
    }

    async fn block_state_hash(&self, ctx: &Context<'_>) -> Result<Option<Block>> {
        let db = db(ctx);

        // blocks
        let epoch_num_blocks = db.get_block_production_epoch_count(None)?;
        let total_num_blocks = db.get_block_production_total_count()?;

        // canonical blocks
        let epoch_num_canonical_blocks = db.get_block_production_canonical_epoch_count(None)?;

        // supercharged
        let epoch_num_supercharged_blocks =
            db.get_block_production_supercharged_epoch_count(None)?;
        let total_num_supercharged_blocks = db.get_block_production_supercharged_total_count()?;

        // all user commands
        let epoch_num_user_commands = db.get_user_commands_epoch_count(None)?;
        let total_num_user_commands = db.get_user_commands_total_count()?;

        // zkapp commands
        let epoch_num_zkapp_commands = db.get_zkapp_commands_epoch_count(None)?;
        let total_num_zkapp_commands = db.get_zkapp_commands_total_count()?;

        // slot produced
        let epoch_num_slots_produced = db.get_epoch_slots_produced_count(None)?;

        if let Some(block) = self.block.as_ref() {
            let block_num_snarks = db
                .get_block_snarks_count(&block.state_hash())?
                .unwrap_or_default();
            let block_num_user_commands = db
                .get_block_user_commands_count(&block.state_hash())?
                .unwrap_or_default();
            let block_num_zkapp_commands = db
                .get_block_zkapp_commands_count(&block.state_hash())?
                .unwrap_or_default();
            let block_num_internal_commands = db
                .get_block_internal_commands_count(&block.state_hash())?
                .unwrap_or_default();

            return Ok(Some(Block {
                block: BlockWithoutCanonicity::new(
                    db,
                    block,
                    self.canonical,
                    [
                        epoch_num_user_commands,
                        total_num_user_commands,
                        epoch_num_zkapp_commands,
                        total_num_zkapp_commands,
                    ],
                ),
                canonical: self.canonical,
                epoch_num_blocks,
                epoch_num_canonical_blocks,
                epoch_num_supercharged_blocks,
                total_num_blocks,
                total_num_supercharged_blocks,
                block_num_snarks,
                block_num_user_commands,
                block_num_zkapp_commands,
                block_num_internal_commands,
                epoch_num_slots_produced,
                num_unique_block_producers_last_n_blocks: None,
            }));
        }

        Ok(None)
    }
}

#[derive(InputObject, Default)]
pub struct FeetransferQueryInput {
    /// Value block height
    block_height: Option<u32>,

    /// Value block state hash
    block_state_hash: Option<BlockQueryInput>,

    /// Value canonical
    canonical: Option<bool>,

    /// Value recipient
    recipient: Option<String>,

    /// Value block height greater than
    #[graphql(name = "blockHeight_gt")]
    pub block_height_gt: Option<u32>,

    /// Value block height greater than or equal to
    #[graphql(name = "blockHeight_gte")]
    pub block_height_gte: Option<u32>,

    /// Value block height less than
    #[graphql(name = "blockHeight_lt")]
    pub block_height_lt: Option<u32>,

    /// Value block height less than or equal to
    #[graphql(name = "blockHeight_lte")]
    pub block_height_lte: Option<u32>,

    /// Value and
    and: Option<Vec<FeetransferQueryInput>>,

    /// Value or
    or: Option<Vec<FeetransferQueryInput>>,
}

#[derive(Default, Enum, Copy, Clone, Eq, PartialEq)]
pub enum FeetransferSortByInput {
    #[graphql(name = "BLOCKHEIGHT_ASC")]
    BlockHeightAsc,

    #[default]
    #[graphql(name = "BLOCKHEIGHT_DESC")]
    BlockHeightDesc,
}

#[derive(Default)]
pub struct FeetransferQueryRoot;

///////////
// impls //
///////////

#[Object]
impl FeetransferQueryRoot {
    #[graphql(cache_control(max_age = 3600))]
    async fn feetransfers(
        &self,
        ctx: &Context<'_>,
        query: Option<FeetransferQueryInput>,
        sort_by: Option<FeetransferSortByInput>,
        #[graphql(default = 100)] limit: usize,
    ) -> Result<Vec<FeetransferWithMeta>> {
        let db = db(ctx);
        let sort_by = sort_by.unwrap_or_default();

        let epoch_num_internal_commands = db.get_internal_commands_epoch_count(None)?;
        let total_num_internal_commands = db.get_internal_commands_total_count()?;

        // state_hash query
        if let Some(state_hash) = query
            .as_ref()
            .and_then(|q| q.block_state_hash.as_ref())
            .and_then(|q| q.state_hash.as_ref())
        {
            // validate state hash
            let state_hash = match StateHash::new(state_hash) {
                Ok(state_hash) => state_hash,
                Err(_) => {
                    return Err(async_graphql::Error::new(format!(
                        "Invalid state hash: {}",
                        state_hash
                    )))
                }
            };

            return Ok(get_fee_transfers_for_state_hash(
                db,
                &query,
                &state_hash,
                sort_by,
                limit,
                epoch_num_internal_commands,
                total_num_internal_commands,
            ));
        }

        // block height bounded query
        if query.as_ref().is_some_and(|q| {
            q.block_height.is_some()
                || q.block_height_gt.is_some()
                || q.block_height_gte.is_some()
                || q.block_height_lt.is_some()
                || q.block_height_lte.is_some()
        }) {
            return Ok(block_height_bound_query_handler(
                db,
                query.as_ref(),
                sort_by,
                limit,
                epoch_num_internal_commands,
                total_num_internal_commands,
            )?);
        }

        // recipient query
        if let Some(recipient) = query.as_ref().and_then(|q| q.recipient.as_ref()) {
            // validate recipient
            let recipient = match PublicKey::new(recipient) {
                Ok(recipient) => recipient,
                Err(_) => {
                    return Err(async_graphql::Error::new(format!(
                        "Invalid recipient public key: {}",
                        recipient
                    )))
                }
            };

            return Ok(recipient_query_handler(
                db,
                query.as_ref(),
                recipient,
                sort_by,
                limit,
                epoch_num_internal_commands,
                total_num_internal_commands,
            )?);
        }

        default_query_handler(
            db,
            query,
            sort_by,
            limit,
            epoch_num_internal_commands,
            total_num_internal_commands,
        )
    }
}

impl FeetransferQueryInput {
    pub fn matches(&self, ft: &FeetransferWithMeta) -> bool {
        let Self {
            block_height_gt,
            block_height_gte,
            block_height_lt,
            block_height_lte,
            block_height,
            block_state_hash,
            canonical,
            recipient,
            and,
            or,
        } = self;

        if let Some(canonical) = canonical.as_ref() {
            if ft.canonical != *canonical {
                return false;
            }
        }

        if let Some(recipient) = recipient.as_ref() {
            if ft.feetransfer.recipient != *recipient {
                return false;
            }
        }

        let pcb = ft.block.as_ref().expect("block will exist");
        let blockchain_length = pcb.blockchain_length();

        if let Some(height) = block_height.as_ref() {
            if blockchain_length != *height {
                return false;
            }
        }

        // block_height_gt(e) & block_height_lt(e)
        if let Some(height) = block_height_gt {
            if blockchain_length <= *height {
                return false;
            }
        }

        if let Some(height) = block_height_gte {
            if blockchain_length < *height {
                return false;
            }
        }

        if let Some(height) = block_height_lt {
            if blockchain_length >= *height {
                return false;
            }
        }

        if let Some(height) = block_height_lte {
            if blockchain_length > *height {
                return false;
            }
        }

        if let Some(block_query_input) = block_state_hash.as_ref() {
            if let Some(state_hash) = block_query_input.state_hash.as_ref() {
                if ft.feetransfer.state_hash != *state_hash {
                    return false;
                }
            }
        }

        if let Some(query) = and.as_ref() {
            if !(query.iter().all(|and| and.matches(ft))) {
                return false;
            }
        }

        if let Some(query) = or.as_ref() {
            if !query.is_empty() && !(query.iter().any(|or| or.matches(ft))) {
                return false;
            }
        }

        true
    }
}

/////////////////
// conversions //
/////////////////

impl From<(DbInternalCommandWithData, u32, u32)> for Feetransfer {
    fn from(int_cmd: (DbInternalCommandWithData, u32, u32)) -> Self {
        match int_cmd.0 {
            DbInternalCommandWithData::FeeTransfer {
                receiver,
                amount,
                state_hash,
                kind,
                date_time,
                block_height,
                ..
            } => Self {
                state_hash: state_hash.0,
                fee: amount,
                recipient: receiver.0,
                feetransfer_kind: kind.to_string(),
                block_height,
                date_time: millis_to_iso_date_string(date_time),
                epoch_num_internal_commands: int_cmd.1,
                total_num_internal_commands: int_cmd.2,
            },
            DbInternalCommandWithData::Coinbase {
                receiver,
                amount,
                state_hash,
                kind,
                date_time,
                block_height,
            } => Self {
                state_hash: state_hash.0,
                fee: amount,
                recipient: receiver.0,
                feetransfer_kind: kind.to_string(),
                block_height,
                date_time: millis_to_iso_date_string(date_time),
                epoch_num_internal_commands: int_cmd.1,
                total_num_internal_commands: int_cmd.2,
            },
        }
    }
}

/////////////
// helpers //
/////////////

fn default_query_handler(
    db: &Arc<IndexerStore>,
    query: Option<FeetransferQueryInput>,
    sort_by: FeetransferSortByInput,
    limit: usize,
    epoch_num_internal_commands: u32,
    total_num_internal_commands: u32,
) -> Result<Vec<FeetransferWithMeta>> {
    let mut fee_transfers = vec![];
    let mode = match sort_by {
        FeetransferSortByInput::BlockHeightAsc => IteratorMode::Start,
        FeetransferSortByInput::BlockHeightDesc => IteratorMode::End,
    };

    for (key, value) in db.internal_commands_block_height_iterator(mode).flatten() {
        if fee_transfers.len() >= limit {
            break;
        }

        let height = from_be_bytes(key[..U32_LEN].to_vec());
        let state_hash = StateHash::from_bytes(&key[U32_LEN..][..StateHash::LEN])?;
        let canonical = get_block_canonicity(db, &state_hash);

        if let Some(q) = query.as_ref() {
            if block_out_of_bounds(height, q) {
                break;
            }

            if let Some(query_canonicity) = q.canonical {
                if canonical != query_canonicity {
                    continue;
                }
            }
        }

        let pcb = get_block(db, &state_hash);
        let ft = Feetransfer::from((
            serde_json::from_slice::<DbInternalCommandWithData>(&value)?,
            epoch_num_internal_commands,
            total_num_internal_commands,
        ));
        let feetransfer_with_meta = FeetransferWithMeta {
            canonical,
            feetransfer: ft,
            block: Some(pcb),
        };

        if query
            .as_ref()
            .is_none_or(|q| q.matches(&feetransfer_with_meta))
        {
            fee_transfers.push(feetransfer_with_meta);
        }
    }

    Ok(fee_transfers)
}

fn get_fee_transfers_for_state_hash(
    db: &Arc<IndexerStore>,
    query: &Option<FeetransferQueryInput>,
    state_hash: &StateHash,
    sort_by: FeetransferSortByInput,
    limit: usize,
    epoch_num_internal_commands: u32,
    total_num_internal_commands: u32,
) -> Vec<FeetransferWithMeta> {
    let canonical = get_block_canonicity(db, state_hash);
    if let Some(query_canonicity) = query.as_ref().and_then(|q| q.canonical) {
        if canonical != query_canonicity {
            return vec![];
        }
    }

    match db.get_internal_commands(state_hash) {
        Ok(internal_commands) => {
            let pcb = match db.get_block(state_hash) {
                Ok(Some(pcb)) => pcb.0,
                _ => return vec![],
            };

            let mut internal_commands: Vec<FeetransferWithMeta> = internal_commands
                .into_iter()
                .map(|ft| FeetransferWithMeta {
                    canonical,
                    feetransfer: Feetransfer::from((
                        ft,
                        epoch_num_internal_commands,
                        total_num_internal_commands,
                    )),
                    block: Some(pcb.clone()),
                })
                .filter(|ft| query.as_ref().is_none_or(|q| q.matches(ft)))
                .collect();

            match sort_by {
                FeetransferSortByInput::BlockHeightAsc => {
                    internal_commands.sort_by(|a, b| {
                        a.feetransfer.block_height.cmp(&b.feetransfer.block_height)
                    });
                }
                FeetransferSortByInput::BlockHeightDesc => {
                    internal_commands.sort_by(|a, b| {
                        b.feetransfer.block_height.cmp(&a.feetransfer.block_height)
                    });
                }
            }

            internal_commands.truncate(limit);
            internal_commands
        }
        Err(_) => vec![],
    }
}

fn block_height_bound_query_handler(
    db: &Arc<IndexerStore>,
    query: Option<&FeetransferQueryInput>,
    sort_by: FeetransferSortByInput,
    limit: usize,
    epoch_num_internal_commands: u32,
    total_num_internal_commands: u32,
) -> anyhow::Result<Vec<FeetransferWithMeta>> {
    // height bounds
    let (min, max) = {
        let FeetransferQueryInput {
            block_height_gt,
            block_height_gte,
            block_height_lt,
            block_height_lte,
            block_height,
            ..
        } = query.expect("query will contain a value");

        if let Some(block_height) = block_height {
            (*block_height, *block_height + 1)
        } else {
            let min_bound = match (*block_height_gte, *block_height_gt) {
                (Some(gte), Some(gt)) => gte.max(gt + 1),
                (Some(gte), None) => gte,
                (None, Some(gt)) => gt + 1,
                (None, None) => 1,
            };

            let max_bound = match (*block_height_lte, *block_height_lt) {
                (Some(lte), Some(lt)) => lte.min(lt - 1),
                (Some(lte), None) => lte,
                (None, Some(lt)) => lt - 1,
                (None, None) => db.get_best_block_height()?.unwrap(),
            };

            (min_bound, max_bound + 1)
        }
    };

    // iterator
    let iter = match sort_by {
        FeetransferSortByInput::BlockHeightAsc => db.internal_commands_block_height_iterator(
            IteratorMode::From(&min.to_be_bytes(), Direction::Forward),
        ),
        FeetransferSortByInput::BlockHeightDesc => db.internal_commands_block_height_iterator(
            IteratorMode::From(&max.to_be_bytes(), Direction::Reverse),
        ),
    };

    // iterate
    let mut fee_transfers = vec![];
    for (key, value) in iter.flatten() {
        if fee_transfers.len() >= limit {
            break;
        }

        // avoid deserializing internal command & PCB if possible
        let state_hash = StateHash::from_bytes(&key[U32_LEN..][..StateHash::LEN])?;
        let canonical = get_block_canonicity(db, &state_hash);

        if let Some(q) = query.as_ref() {
            if block_out_of_bounds(from_be_bytes(key[..U32_LEN].to_vec()), q) {
                break;
            }

            if let Some(query_canonicity) = q.canonical {
                if canonical != query_canonicity {
                    continue;
                }
            }
        }

        let feetransfer_with_meta = FeetransferWithMeta {
            canonical,
            block: Some(get_block(db, &state_hash)),
            feetransfer: Feetransfer::from((
                serde_json::from_slice(&value)?,
                epoch_num_internal_commands,
                total_num_internal_commands,
            )),
        };

        if query
            .as_ref()
            .is_none_or(|q| q.matches(&feetransfer_with_meta))
        {
            fee_transfers.push(feetransfer_with_meta);
        }
    }

    Ok(fee_transfers)
}

fn recipient_query_handler(
    db: &Arc<IndexerStore>,
    query: Option<&FeetransferQueryInput>,
    recipient: PublicKey,
    sort_by: FeetransferSortByInput,
    limit: usize,
    epoch_num_internal_commands: u32,
    total_num_internal_commands: u32,
) -> anyhow::Result<Vec<FeetransferWithMeta>> {
    let pk_bytes = recipient.0.as_bytes().to_vec();

    // iterator
    let iter = match sort_by {
        FeetransferSortByInput::BlockHeightAsc => {
            db.internal_commands_pk_block_height_iterator(recipient, Direction::Forward)
        }
        FeetransferSortByInput::BlockHeightDesc => {
            db.internal_commands_pk_block_height_iterator(recipient, Direction::Reverse)
        }
    };

    // iterate
    let mut fee_transfers = vec![];
    for (key, value) in iter.flatten() {
        if key[..PublicKey::LEN] != pk_bytes || fee_transfers.len() >= limit {
            // we've gone beyond our recipient or limit
            break;
        }

        let state_hash =
            StateHash::from_bytes(&key[PublicKey::LEN..][U32_LEN..][..StateHash::LEN])?;
        let canonical = get_block_canonicity(db, &state_hash);

        if let Some(q) = query.as_ref() {
            let block_height = from_be_bytes(key[PublicKey::LEN..][..U32_LEN].to_vec());
            if block_out_of_bounds(block_height, q) {
                break;
            }

            if let Some(query_canonicity) = q.canonical {
                if canonical != query_canonicity {
                    continue;
                }
            }
        }

        let ft = FeetransferWithMeta {
            canonical,
            block: Some(get_block(db, &state_hash)),
            feetransfer: Feetransfer::from((
                serde_json::from_slice(&value)?,
                epoch_num_internal_commands,
                total_num_internal_commands,
            )),
        };

        if query.as_ref().is_none_or(|q| q.matches(&ft)) {
            fee_transfers.push(ft);
        }
    }

    Ok(fee_transfers)
}

fn block_out_of_bounds(blockchain_length: u32, query: &FeetransferQueryInput) -> bool {
    (query.block_height == Some(blockchain_length))
        || query
            .block_height_gt
            .is_some_and(|gt| blockchain_length <= gt)
        || query
            .block_height_gte
            .is_some_and(|gte| blockchain_length < gte)
        || query
            .block_height_lt
            .is_some_and(|lt| blockchain_length >= lt)
        || query
            .block_height_lte
            .is_some_and(|lte| blockchain_length > lte)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_within_bounds() {
        let query = FeetransferQueryInput {
            block_height_gte: Some(50),
            block_height_gt: None,
            block_height_lte: Some(100),
            block_height_lt: None,
            ..Default::default()
        };
        assert!(!block_out_of_bounds(75, &query));
    }

    #[test]
    fn test_block_height_gte_out_of_bounds() {
        let query = FeetransferQueryInput {
            block_height_gte: Some(100),
            block_height_gt: None,
            block_height_lte: Some(200),
            block_height_lt: None,
            ..Default::default()
        };
        assert!(block_out_of_bounds(99, &query));
    }

    #[test]
    fn test_block_height_gt_out_of_bounds() {
        let query = FeetransferQueryInput {
            block_height_gte: None,
            block_height_gt: Some(100),
            block_height_lte: Some(200),
            block_height_lt: None,
            ..Default::default()
        };
        assert!(block_out_of_bounds(100, &query));
    }

    #[test]
    fn test_block_height_lte_out_of_bounds() {
        let query = FeetransferQueryInput {
            block_height_gte: Some(50),
            block_height_gt: None,
            block_height_lte: Some(75),
            block_height_lt: None,
            ..Default::default()
        };
        assert!(block_out_of_bounds(76, &query));
    }

    #[test]
    fn test_block_height_lt_out_of_bounds() {
        let query = FeetransferQueryInput {
            block_height_gte: Some(50),
            block_height_gt: None,
            block_height_lte: None,
            block_height_lt: Some(100),
            ..Default::default()
        };
        assert!(block_out_of_bounds(100, &query));
    }

    #[test]
    fn test_no_bounds_specified() {
        let query = FeetransferQueryInput {
            block_height_gte: None,
            block_height_gt: None,
            block_height_lte: None,
            block_height_lt: None,
            ..Default::default()
        };
        assert!(!block_out_of_bounds(100, &query));
    }
}
