//! GraphQL `internalCommands` endpoint

use super::{
    blocks::block::{Block, BlockWithoutCanonicity},
    gen::BlockQueryInput,
    get_block, get_block_canonicity,
    pk::RecipientPK,
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
pub struct InternalCommand {
    /// Value block state hash
    pub state_hash: String,

    /// Value fee (nanomina)
    pub fee: u64,

    /// Value fee transfer recipient
    #[graphql(flatten)]
    pub recipient: RecipientPK,

    /// Value block height
    pub block_height: u32,

    /// Value block date time
    pub date_time: String,

    /// Value fee transfer kind
    #[graphql(name = "type")]
    pub internal_command_kind: String,

    /// Value epoch internal commands count
    #[graphql(name = "epoch_num_internal_commands")]
    epoch_num_internal_commands: u32,

    /// Value total internal commands count
    #[graphql(name = "total_num_internal_commands")]
    total_num_internal_commands: u32,
}

#[derive(Debug)]
pub struct InternalCommandWithMeta {
    /// Value canonicity
    pub canonical: bool,

    /// Value optional block
    pub block: Option<PrecomputedBlock>,

    /// Value internal command
    pub internal_command: InternalCommand,
}

#[Object]
impl InternalCommandWithMeta {
    async fn canonical(&self) -> bool {
        self.canonical
    }

    #[graphql(flatten)]
    async fn internal_command(&self) -> &InternalCommand {
        &self.internal_command
    }

    async fn block_state_hash(&self, ctx: &Context<'_>) -> Result<Option<Block>> {
        let db = db(ctx);

        // blocks
        let epoch_num_blocks = db.get_block_production_epoch_count(None, None)?;
        let total_num_blocks = db.get_block_production_total_count()?;

        // canonical blocks
        let epoch_num_canonical_blocks =
            db.get_block_production_canonical_epoch_count(None, None)?;

        // supercharged
        let epoch_num_supercharged_blocks =
            db.get_block_production_supercharged_epoch_count(None, None)?;
        let total_num_supercharged_blocks = db.get_block_production_supercharged_total_count()?;

        // all user commands
        let epoch_num_user_commands = db.get_user_commands_epoch_count(None, None)?;
        let total_num_user_commands = db.get_user_commands_total_count()?;

        // zkapp commands
        let epoch_num_zkapp_commands = db.get_zkapp_commands_epoch_count(None, None)?;
        let total_num_zkapp_commands = db.get_zkapp_commands_total_count()?;

        // slot produced
        let epoch_num_slots_produced = db.get_epoch_slots_produced_count(None, None)?;

        if let Some(block) = self.block.as_ref() {
            let state_hash = block.state_hash();
            let block_num_snarks = db.get_block_snarks_count(&state_hash)?.unwrap_or_default();
            let block_num_user_commands = db
                .get_block_user_commands_count(&state_hash)?
                .unwrap_or_default();
            let block_num_zkapp_commands = db
                .get_block_zkapp_commands_count(&state_hash)?
                .unwrap_or_default();
            let block_num_internal_commands = db
                .get_block_internal_commands_count(&state_hash)?
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
pub struct InternalCommandQueryInput {
    /// Value block height
    block_height: Option<u32>,

    /// Value block state hash
    block_state_hash: Option<BlockQueryInput>,

    /// Value canonical
    canonical: Option<bool>,

    /// Value recipient
    recipient: Option<String>,

    /// Value block height greater than
    pub block_height_gt: Option<u32>,

    /// Value block height greater than or equal to
    pub block_height_gte: Option<u32>,

    /// Value block height less than
    pub block_height_lt: Option<u32>,

    /// Value block height less than or equal to
    pub block_height_lte: Option<u32>,

    /// Value and
    and: Option<Vec<InternalCommandQueryInput>>,

    /// Value or
    or: Option<Vec<InternalCommandQueryInput>>,
}

#[derive(Default, Enum, Copy, Clone, Eq, PartialEq)]
pub enum InternalCommandSortByInput {
    #[default]
    BlockHeightDesc,
    BlockHeightAsc,
}

#[derive(Default)]
pub struct InternalCommandQueryRoot;

///////////
// impls //
///////////

#[Object]
impl InternalCommandQueryRoot {
    #[graphql(cache_control(max_age = 3600))]
    async fn internal_commands(
        &self,
        ctx: &Context<'_>,
        query: Option<InternalCommandQueryInput>,
        sort_by: Option<InternalCommandSortByInput>,
        #[graphql(default = 100)] limit: usize,
    ) -> Result<Vec<InternalCommandWithMeta>> {
        let db = db(ctx);
        let sort_by = sort_by.unwrap_or_default();

        let epoch_num_internal_commands = db.get_internal_commands_epoch_count(None, None)?;
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

            return Ok(get_internal_commands_for_state_hash(
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

impl InternalCommandQueryInput {
    pub fn matches(&self, cmd: &InternalCommandWithMeta) -> bool {
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
            if cmd.canonical != *canonical {
                return false;
            }
        }

        if let Some(recipient) = recipient.as_ref() {
            if cmd.internal_command.recipient.recipient != *recipient {
                return false;
            }
        }

        let pcb = cmd.block.as_ref().expect("block will exist");
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
                if cmd.internal_command.state_hash != *state_hash {
                    return false;
                }
            }
        }

        if let Some(query) = and.as_ref() {
            if !(query.iter().all(|and| and.matches(cmd))) {
                return false;
            }
        }

        if let Some(query) = or.as_ref() {
            if !query.is_empty() && !(query.iter().any(|or| or.matches(cmd))) {
                return false;
            }
        }

        true
    }
}

/////////////////
// conversions //
/////////////////

impl InternalCommand {
    fn new(
        db: &Arc<IndexerStore>,
        int_cmd: DbInternalCommandWithData,
        epoch_num_internal_commands: u32,
        total_num_internal_commands: u32,
    ) -> Self {
        match int_cmd {
            DbInternalCommandWithData::FeeTransfer {
                receiver,
                amount,
                state_hash,
                kind,
                date_time,
                block_height,
                ..
            }
            | DbInternalCommandWithData::Coinbase {
                receiver,
                amount,
                state_hash,
                kind,
                date_time,
                block_height,
            } => Self {
                state_hash: state_hash.0,
                fee: amount,
                recipient: RecipientPK::new(db, receiver),
                internal_command_kind: kind.to_string(),
                block_height,
                date_time: millis_to_iso_date_string(date_time),
                epoch_num_internal_commands,
                total_num_internal_commands,
            },
        }
    }
}

/////////////
// helpers //
/////////////

fn default_query_handler(
    db: &Arc<IndexerStore>,
    query: Option<InternalCommandQueryInput>,
    sort_by: InternalCommandSortByInput,
    limit: usize,
    epoch_num_internal_commands: u32,
    total_num_internal_commands: u32,
) -> Result<Vec<InternalCommandWithMeta>> {
    let mut internal_commands = vec![];
    let mode = match sort_by {
        InternalCommandSortByInput::BlockHeightAsc => IteratorMode::Start,
        InternalCommandSortByInput::BlockHeightDesc => IteratorMode::End,
    };

    for (key, value) in db.internal_commands_block_height_iterator(mode).flatten() {
        if internal_commands.len() >= limit {
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
        let cmd = InternalCommand::new(
            db,
            serde_json::from_slice::<DbInternalCommandWithData>(&value)?,
            epoch_num_internal_commands,
            total_num_internal_commands,
        );
        let internal_command_with_meta = InternalCommandWithMeta {
            canonical,
            internal_command: cmd,
            block: Some(pcb),
        };

        if query
            .as_ref()
            .is_none_or(|q| q.matches(&internal_command_with_meta))
        {
            internal_commands.push(internal_command_with_meta);
        }
    }

    Ok(internal_commands)
}

fn get_internal_commands_for_state_hash(
    db: &Arc<IndexerStore>,
    query: &Option<InternalCommandQueryInput>,
    state_hash: &StateHash,
    sort_by: InternalCommandSortByInput,
    limit: usize,
    epoch_num_internal_commands: u32,
    total_num_internal_commands: u32,
) -> Vec<InternalCommandWithMeta> {
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

            let mut internal_commands: Vec<InternalCommandWithMeta> = internal_commands
                .into_iter()
                .map(|cmd| InternalCommandWithMeta {
                    canonical,
                    internal_command: InternalCommand::new(
                        db,
                        cmd,
                        epoch_num_internal_commands,
                        total_num_internal_commands,
                    ),
                    block: Some(pcb.clone()),
                })
                .filter(|cmd| query.as_ref().is_none_or(|q| q.matches(cmd)))
                .collect();

            match sort_by {
                InternalCommandSortByInput::BlockHeightAsc => {
                    internal_commands.sort_by(|a, b| {
                        a.internal_command
                            .block_height
                            .cmp(&b.internal_command.block_height)
                    });
                }
                InternalCommandSortByInput::BlockHeightDesc => {
                    internal_commands.sort_by(|a, b| {
                        b.internal_command
                            .block_height
                            .cmp(&a.internal_command.block_height)
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
    query: Option<&InternalCommandQueryInput>,
    sort_by: InternalCommandSortByInput,
    limit: usize,
    epoch_num_internal_commands: u32,
    total_num_internal_commands: u32,
) -> anyhow::Result<Vec<InternalCommandWithMeta>> {
    // height bounds
    let (min, max) = {
        let InternalCommandQueryInput {
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
        InternalCommandSortByInput::BlockHeightAsc => db.internal_commands_block_height_iterator(
            IteratorMode::From(&min.to_be_bytes(), Direction::Forward),
        ),
        InternalCommandSortByInput::BlockHeightDesc => db.internal_commands_block_height_iterator(
            IteratorMode::From(&max.to_be_bytes(), Direction::Reverse),
        ),
    };

    // iterate
    let mut internal_commands = vec![];
    for (key, value) in iter.flatten() {
        if internal_commands.len() >= limit {
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

        let internal_command_with_meta = InternalCommandWithMeta {
            canonical,
            block: Some(get_block(db, &state_hash)),
            internal_command: InternalCommand::new(
                db,
                serde_json::from_slice(&value)?,
                epoch_num_internal_commands,
                total_num_internal_commands,
            ),
        };

        if query
            .as_ref()
            .is_none_or(|q| q.matches(&internal_command_with_meta))
        {
            internal_commands.push(internal_command_with_meta);
        }
    }

    Ok(internal_commands)
}

fn recipient_query_handler(
    db: &Arc<IndexerStore>,
    query: Option<&InternalCommandQueryInput>,
    recipient: PublicKey,
    sort_by: InternalCommandSortByInput,
    limit: usize,
    epoch_num_internal_commands: u32,
    total_num_internal_commands: u32,
) -> anyhow::Result<Vec<InternalCommandWithMeta>> {
    let pk_bytes = recipient.0.as_bytes().to_vec();

    // iterator
    let iter = match sort_by {
        InternalCommandSortByInput::BlockHeightAsc => {
            db.internal_commands_pk_block_height_iterator(recipient, Direction::Forward)
        }
        InternalCommandSortByInput::BlockHeightDesc => {
            db.internal_commands_pk_block_height_iterator(recipient, Direction::Reverse)
        }
    };

    // iterate
    let mut internal_commands = vec![];
    for (key, value) in iter.flatten() {
        if key[..PublicKey::LEN] != pk_bytes || internal_commands.len() >= limit {
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

        let cmd = InternalCommandWithMeta {
            canonical,
            block: Some(get_block(db, &state_hash)),
            internal_command: InternalCommand::new(
                db,
                serde_json::from_slice(&value)?,
                epoch_num_internal_commands,
                total_num_internal_commands,
            ),
        };

        if query.as_ref().is_none_or(|q| q.matches(&cmd)) {
            internal_commands.push(cmd);
        }
    }

    Ok(internal_commands)
}

fn block_out_of_bounds(blockchain_length: u32, query: &InternalCommandQueryInput) -> bool {
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
        let query = InternalCommandQueryInput {
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
        let query = InternalCommandQueryInput {
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
        let query = InternalCommandQueryInput {
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
        let query = InternalCommandQueryInput {
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
        let query = InternalCommandQueryInput {
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
        let query = InternalCommandQueryInput {
            block_height_gte: None,
            block_height_gt: None,
            block_height_lte: None,
            block_height_lt: None,
            ..Default::default()
        };
        assert!(!block_out_of_bounds(100, &query));
    }
}
