use super::blocks::{Block, BlockWithCanonicity};
use crate::{
    block::{precomputed::PrecomputedBlock, store::BlockStore, BlockHash},
    canonicity::{store::CanonicityStore, Canonicity},
    command::{internal::InternalCommandWithData, store::CommandStore},
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
}

pub struct FeetransferWithMeta {
    /// Value canonicity
    pub canonical: bool,
    /// Value block height
    pub block_height: u32,
    /// Value date time
    pub date_time: String,
    /// Value optional block
    pub block: PrecomputedBlock,
    /// Value feetranser
    pub feetransfer: Feetransfer,
}

#[Object]
impl FeetransferWithMeta {
    async fn canonicity(&self) -> bool {
        self.canonical
    }

    async fn block_height(&self) -> u32 {
        self.block_height
    }

    #[graphql(flatten)]
    async fn feetransfer(&self) -> &Feetransfer {
        &self.feetransfer
    }

    async fn block_state_hash(&self) -> Option<BlockWithCanonicity> {
        Some(BlockWithCanonicity {
            block: Block::from(self.block.clone()),
            canonical: self.canonical,
        })
    }
}

async fn fetch_block_by_state_hash(
    db: &Arc<IndexerStore>,
    state_hash: String,
) -> Result<Option<BlockWithCanonicity>> {
    let state_hash = BlockHash::from(state_hash);
    let pcb = match db.get_block(&state_hash)? {
        Some(pcb) => pcb,
        None => return Ok(None),
    };
    let block = Block::from(pcb);
    let canonical = db
        .get_block_canonicity(&state_hash)?
        .map(|status| matches!(status, Canonicity::Canonical))
        .unwrap_or(false);
    Ok(Some(BlockWithCanonicity { block, canonical }))
}

#[derive(InputObject)]
pub struct FeetransferQueryInput {
    state_hash: Option<String>,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum FeetransferSortByInput {
    FeeDesc,
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
        // TODO: Pick a default state_hash
        let state_hash = query
            .unwrap()
            .state_hash
            .clone()
            .unwrap_or("asdf".to_string());

        let state_hash = BlockHash::from(state_hash);
        let pcb = match db.get_block(&state_hash)? {
            Some(pcb) => pcb,
            None => return Ok(None),
        };
        let canonical = db
            .get_block_canonicity(&state_hash)?
            .map(|status| matches!(status, Canonicity::Canonical))
            .unwrap_or(false);

        match db.get_internal_commands(&BlockHash::from(state_hash)) {
            Ok(internal_commands) => {
                let mut internal_commands: Vec<FeetransferWithMeta> = internal_commands
                    .into_iter()
                    .map(|ft| FeetransferWithMeta {
                        canonical,
                        block_height: pcb.blockchain_length,
                        feetransfer: Feetransfer::from(ft),
                        date_time: millis_to_date_string(pcb.timestamp().try_into().unwrap()),
                        block: pcb.clone(),
                    })
                    .collect();
                internal_commands.truncate(limit);
                Ok(Some(internal_commands))
            }
            Err(_) => Ok(None),
        }
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
                ..
            } => Self {
                state_hash: state_hash.0,
                fee: amount,
                recipient: receiver.0,
                feetransfer_kind: kind.to_string(),
            },
            InternalCommandWithData::Coinbase {
                receiver,
                amount,
                state_hash,
                kind,
            } => Self {
                state_hash: state_hash.0,
                fee: amount,
                recipient: receiver.0,
                feetransfer_kind: kind.to_string(),
            },
        }
    }
}
