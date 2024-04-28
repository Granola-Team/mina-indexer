use super::{
    blocks::{Block, BlockWithCanonicity},
    get_block_canonicity,
};
use crate::{
    block::{precomputed::PrecomputedBlock, store::BlockStore, BlockHash},
    canonicity::{store::CanonicityStore, Canonicity},
    command::{internal::InternalCommandWithData, store::CommandStore},
    constants::*,
    store::IndexerStore,
    web::graphql::db,
};
use async_graphql::{Context, Enum, InputObject, Object, Result, SimpleObject};
use std::sync::Arc;

#[derive(SimpleObject, Debug)]
pub struct Feetransfer {
    pub state_hash: String,
    pub fee: u64,
    pub recipient: String,
    #[graphql(name = "type")]
    pub feetransfer_kind: String,
    pub block_height: u32,
    pub date_time: String,
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

    async fn block_state_hash(&self) -> Option<BlockWithCanonicity> {
        self.block.clone().map(|block| BlockWithCanonicity {
            block: Block::from(block),
            canonical: self.canonical,
        })
    }
}

#[derive(InputObject, Clone)]
pub struct FeetransferQueryInput {
    state_hash: Option<String>,
    canonical: Option<bool>,
    and: Option<Vec<FeetransferQueryInput>>,
    or: Option<Vec<FeetransferQueryInput>>,
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

#[Object]
impl FeetransferQueryRoot {
    async fn feetransfers<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        query: Option<FeetransferQueryInput>,
        sort_by: Option<FeetransferSortByInput>,
        #[graphql(default = 100)] limit: usize,
    ) -> Result<Vec<FeetransferWithMeta>> {
        let db = db(ctx);
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
) -> Result<Vec<FeetransferWithMeta>> {
    let mut fee_transfers: Vec<FeetransferWithMeta> = Vec::with_capacity(limit);
    let mode = if let Some(FeetransferSortByInput::BlockHeightAsc) = sort_by {
        speedb::IteratorMode::Start
    } else {
        speedb::IteratorMode::End
    };

    for entry in db.get_internal_commands_interator(mode) {
        let (_, value) = entry?;
        let internal_command = serde_json::from_slice::<InternalCommandWithData>(&value)?;
        let ft = Feetransfer::from(internal_command);
        let state_hash = ft.state_hash.clone();
        let feetransfer_with_meta = FeetransferWithMeta {
            canonical: get_block_canonicity(db, &state_hash),
            feetransfer: ft,
            block: None,
        };

        if query
            .as_ref()
            .map_or(true, |q| q.matches(&feetransfer_with_meta))
        {
            fee_transfers.push(feetransfer_with_meta);
        }

        if fee_transfers.len() >= limit {
            break;
        }
    }
    Ok(fee_transfers)
}

fn get_fee_transfers_for_state_hash(
    db: &Arc<IndexerStore>,
    state_hash: &BlockHash,
    sort_by: Option<FeetransferSortByInput>,
    limit: usize,
) -> Vec<FeetransferWithMeta> {
    let pcb = match db.get_block(state_hash) {
        Ok(Some(pcb)) => pcb,
        _ => return vec![],
    };
    let canonical = match db.get_block_canonicity(state_hash) {
        Ok(Some(canonicity)) => matches!(canonicity, Canonicity::Canonical),
        _ => false,
    };

    match db.get_internal_commands(state_hash) {
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
            internal_commands
        }
        Err(_) => vec![],
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
                date_time: millis_to_iso_date_string(date_time),
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
                date_time: millis_to_iso_date_string(date_time),
            },
        }
    }
}

impl FeetransferQueryInput {
    pub fn matches(&self, ft: &FeetransferWithMeta) -> bool {
        let mut matches = true;

        if let Some(state_hash) = &self.state_hash {
            matches = matches && &ft.feetransfer.state_hash == state_hash;
        }

        if let Some(canonical) = &self.canonical {
            matches = matches && &ft.canonical == canonical;
        }

        if let Some(query) = &self.and {
            matches = matches && query.iter().all(|and| and.matches(ft));
        }

        if let Some(query) = &self.or {
            if !query.is_empty() {
                matches = matches && query.iter().any(|or| or.matches(ft));
            }
        }
        matches
    }
}
