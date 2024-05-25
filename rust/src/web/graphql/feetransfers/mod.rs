use super::{
    blocks::{Block, BlockWithCanonicity},
    gen::BlockQueryInput,
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

    async fn block_state_hash<'ctx>(
        &self,
        ctx: &Context<'ctx>,
    ) -> Result<Option<BlockWithCanonicity>> {
        let db = db(ctx);
        let total_num_blocks = db.get_block_production_total_count()?;
        Ok(self.block.clone().map(|block| BlockWithCanonicity {
            block: Block::new(block, self.canonical),
            canonical: self.canonical,
            total_num_blocks,
        }))
    }
}

#[derive(InputObject)]
pub struct FeetransferQueryInput {
    /// Value block height
    block_height: Option<u32>,
    /// Value block state hash
    block_state_hash: Option<BlockQueryInput>,
    /// Value canonical
    canonical: Option<bool>,
    ///Value recipient
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

        //state_hash
        if let Some(state_hash) = query
            .as_ref()
            .and_then(|f| f.block_state_hash.as_ref())
            .and_then(|f| f.state_hash.clone())
        {
            return Ok(get_fee_transfers_for_state_hash(
                db,
                &state_hash.into(),
                sort_by,
                limit,
            ));
        }
        // block height bounded query
        if query.as_ref().map_or(false, |q| {
            q.block_height_gt.is_some()
                || q.block_height_gte.is_some()
                || q.block_height_lt.is_some()
                || q.block_height_lte.is_some()
        }) {
            let mut feetransfers: Vec<FeetransferWithMeta> = Vec::with_capacity(limit);
            let (min, max) = match query.as_ref() {
                Some(feetranser_query_input) => {
                    let FeetransferQueryInput {
                        block_height_gt,
                        block_height_gte,
                        block_height_lt,
                        block_height_lte,
                        ..
                    } = feetranser_query_input;
                    (
                        // min = max of the gt(e) heights or 1
                        block_height_gt
                            .map(|h| h.max(block_height_gte.unwrap_or_default()))
                            .unwrap_or(1),
                        // max = max of the lt(e) heights or best tip height
                        block_height_lt
                            .map(|h| h.max(block_height_lte.unwrap_or_default()))
                            .unwrap_or(db.get_best_block()?.unwrap().blockchain_length())
                            .min(db.get_best_block()?.unwrap().blockchain_length()),
                    )
                }
                None => (1, db.get_best_block()?.unwrap().blockchain_length()),
            };

            let block_heights: Vec<u32> = (min..=max).collect();
            for height in block_heights {
                for block in db.get_blocks_at_height(height)? {
                    let canonical = get_block_canonicity(db, &block.state_hash().0);
                    let internal_cmds = InternalCommandWithData::from_precomputed(&block);
                    for internal_cmd in internal_cmds {
                        let ft = Feetransfer::from(internal_cmd);
                        let feetransfer_with_meta = FeetransferWithMeta {
                            canonical,
                            feetransfer: ft,
                            block: Some(block.clone()),
                        };
                        if query
                            .as_ref()
                            .map_or(true, |q| q.matches(&feetransfer_with_meta))
                        {
                            feetransfers.push(feetransfer_with_meta);
                        }
                    }
                }
            }
            let sort_by = sort_by.unwrap_or(FeetransferSortByInput::BlockHeightDesc);
            if sort_by == FeetransferSortByInput::BlockHeightDesc {
                feetransfers.reverse()
            }

            feetransfers.truncate(limit);
            return Ok(feetransfers);
        }

        // recipient
        if let Some(recipient) = query.as_ref().and_then(|q| q.recipient.clone()) {
            let mut fee_transfers: Vec<FeetransferWithMeta> = db
                .get_internal_commands_public_key(&recipient.into())?
                .into_iter()
                .map(|internal_command| {
                    let ft = Feetransfer::from(internal_command);
                    let state_hash = ft.state_hash.clone();
                    let pcb = db
                        .get_block(&BlockHash::from(state_hash.clone()))
                        .unwrap()
                        .unwrap();
                    let canonical = get_block_canonicity(db, &state_hash);
                    FeetransferWithMeta {
                        canonical,
                        feetransfer: ft,
                        block: Some(pcb),
                    }
                })
                .filter(|ft| ft.feetransfer.feetransfer_kind != "Coinbase")
                .collect();
            fee_transfers.truncate(limit);
            return Ok(fee_transfers);
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
        let canonical = get_block_canonicity(db, &state_hash);
        let pcb = db
            .get_block(&BlockHash::from(state_hash.clone()))
            .unwrap()
            .unwrap();
        let feetransfer_with_meta = FeetransferWithMeta {
            canonical,
            feetransfer: ft,
            block: Some(pcb),
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
                .filter(|ft| ft.feetransfer.feetransfer_kind != "Coinbase")
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
        let Self {
            block_height_gt,
            block_height_gte,
            block_height_lt,
            block_height_lte,
            ..
        } = self;
        let pcb = ft.block.as_ref().unwrap();
        let blockchain_length = pcb.blockchain_length();
        let mut matches = true;
        // block_height_gt(e) & block_height_lt(e)
        if let Some(height) = block_height_gt {
            matches &= blockchain_length > *height;
        }
        if let Some(height) = block_height_gte {
            matches &= blockchain_length >= *height;
        }
        if let Some(height) = block_height_lt {
            matches &= blockchain_length < *height;
        }
        if let Some(height) = block_height_lte {
            matches &= blockchain_length <= *height;
        }

        if let Some(block_query_input) = &self.block_state_hash {
            if let Some(state_hash) = &block_query_input.state_hash {
                matches &= &ft.feetransfer.state_hash == state_hash;
            }
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
