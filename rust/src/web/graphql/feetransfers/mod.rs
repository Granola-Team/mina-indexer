use super::blocks::{Block, BlockWithCanonicity};
use crate::{
    block::{precomputed::PrecomputedBlock, store::BlockStore, BlockHash},
    canonicity::{store::CanonicityStore, Canonicity},
    command::{internal::InternalCommandWithData, store::CommandStore},
    constants::MAINNET_GENESIS_HASH,
    store::IndexerStore,
};
use async_graphql::{Context, Enum, InputObject, Object, Result, SimpleObject};
use chrono::{DateTime, SecondsFormat};
use std::sync::Arc;

#[derive(SimpleObject)]
pub struct Feetransfer {
    pub state_hash: String,
    pub fee: u64,
    pub recipient: String,
    #[graphql(name = "type")]
    pub feetransfer_kind: String,
    pub block_height: u32,
    pub date_time: String,
}

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

    async fn block_state_hash(&self) -> Option<BlockWithCanonicity> {
        match self.block.clone() {
            Some(block) => Some(BlockWithCanonicity {
                block: Block::from(block),
                canonical: self.canonical,
            }),
            None => None,
        }
    }
}

#[derive(InputObject, Clone)]
pub struct FeetransferQueryInput {
    state_hash: Option<String>,
    canonical: Option<bool>,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum FeetransferSortByInput {
    #[graphql(name = "BLOCKHEIGHT_ASC")]
    BlockHeightAsc,
    #[graphql(name = "BLOCKHEIGHT_DESC")]
    BlockHeightDesc,
}

#[derive(Default)]
pub struct FeetransferQueryRoot;

/// convert epoch millis to an ISO 8601 formatted date
fn millis_to_date_string(millis: i64) -> String {
    let date_time = DateTime::from_timestamp_millis(millis).unwrap();
    date_time.to_rfc3339_opts(SecondsFormat::Millis, true)
}

#[Object]
impl FeetransferQueryRoot {
    async fn feetransfers<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        query: Option<FeetransferQueryInput>,
        sort_by: Option<FeetransferSortByInput>,
        limit: Option<usize>,
    ) -> Result<Option<Vec<FeetransferWithMeta>>> {
        let db = ctx
            .data::<Arc<IndexerStore>>()
            .expect("db to be in context");
        let limit = limit.unwrap_or(100);

        let has_state_hash = query.as_ref().map_or(false, |q| q.state_hash.is_some());

        if has_state_hash {
            let state_hash = query
                .as_ref()
                .and_then(|q| q.state_hash.as_ref())
                .map_or(MAINNET_GENESIS_HASH, |s| s);
            let state_hash = BlockHash::from(state_hash);
            return Ok(get_fee_transfers_for_state_hash(
                db,
                &state_hash,
                sort_by,
                limit,
            ));
        }
        get_fee_transfers(db, query, sort_by, limit)
    }
}

fn get_fee_transfers(
    db: &Arc<IndexerStore>,
    query: Option<FeetransferQueryInput>,
    sort_by: Option<FeetransferSortByInput>,
    limit: usize,
) -> Result<Option<Vec<FeetransferWithMeta>>> {
    let mut fee_transfers: Vec<FeetransferWithMeta> = Vec::new();
    let mode: speedb::IteratorMode = match sort_by {
        Some(FeetransferSortByInput::BlockHeightAsc) => speedb::IteratorMode::Start,
        Some(FeetransferSortByInput::BlockHeightDesc) => speedb::IteratorMode::End,
        None => speedb::IteratorMode::End,
    };
    for entry in db.get_internal_commands_interator(mode) {
        let (_, value) = entry?;
        let internal_command = serde_json::from_slice::<InternalCommandWithData>(&value)?;
        let ft = Feetransfer::from(internal_command);
        let state_hash = ft.state_hash.clone();
        let canonical = db
            .get_block_canonicity(&state_hash.into())?
            .map(|status| matches!(status, Canonicity::Canonical))
            .unwrap_or(false);

        let should_filter = query
            .clone()
            .and_then(|q| q.canonical)
            .and_then(|canonicity_filter| Some(canonicity_filter != canonical))
            .unwrap_or(false);
        if should_filter {
            continue;
        }

        fee_transfers.push(FeetransferWithMeta {
            canonical,
            feetransfer: ft,
            block: None,
        });
        if fee_transfers.len() >= limit {
            break;
        }
    }
    Ok(Some(fee_transfers))
}

fn get_fee_transfers_for_state_hash(
    db: &Arc<IndexerStore>,
    state_hash: &BlockHash,
    sort_by: Option<FeetransferSortByInput>,
    limit: usize,
) -> Option<Vec<FeetransferWithMeta>> {
    let pcb = match db.get_block(&state_hash).ok()? {
        Some(pcb) => pcb,
        None => return None,
    };
    let canonical = db
        .get_block_canonicity(&state_hash)
        .ok()?
        .map(|status| matches!(status, Canonicity::Canonical))
        .unwrap_or(false);

    match db.get_internal_commands(&state_hash) {
        Ok(internal_commands) => {
            let mut internal_commands: Vec<FeetransferWithMeta> = internal_commands
                .into_iter()
                .map(|ft| FeetransferWithMeta {
                    canonical,
                    feetransfer: Feetransfer::from(ft),
                    block: Some(pcb.clone()),
                })
                .collect();

            if let Some(sort_by) = sort_by {
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
            }

            internal_commands.truncate(limit);
            Some(internal_commands)
        }
        Err(_) => None,
    }
}

impl From<InternalCommandWithData> for Feetransfer {
    fn from(int_cmd: InternalCommandWithData) -> Self {
        match int_cmd {
            InternalCommandWithData::FeeTransfer {
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
                date_time: millis_to_date_string(date_time),
            },
            InternalCommandWithData::Coinbase {
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
                date_time: millis_to_date_string(date_time),
            },
        }
    }
}
