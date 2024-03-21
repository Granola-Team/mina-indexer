use crate::{
    block::{precomputed::PrecomputedBlock, store::BlockStore, BlockHash},
    canonicity::{store::CanonicityStore, Canonicity},
    proof_systems::signer::pubkey::CompressedPubKey,
    store::IndexerStore,
};
use async_graphql::{
    Context, EmptyMutation, EmptySubscription, InputObject, Object, Result, Schema, SimpleObject,
};
use chrono::{DateTime, SecondsFormat};
use std::sync::Arc;

#[derive(InputObject)]
pub struct BlockQueryInput {
    state_hash: String,
}

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn block<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        query: Option<BlockQueryInput>,
    ) -> Result<Option<BlockWithCanonicity>> {
        let db = ctx
            .data::<Arc<IndexerStore>>()
            .expect("db to be in context");
        // Choose geneesis block if query is None
        let state_hash = match query {
            Some(query) => BlockHash::from(query.state_hash),
            None => match db.get_canonical_hash_at_height(1)? {
                Some(state_hash) => state_hash,
                None => return Ok(None),
            },
        };
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
}

/// Build the schema for the block endpoints
pub fn build_schema(
    store: Arc<IndexerStore>,
) -> Schema<QueryRoot, EmptyMutation, EmptySubscription> {
    Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
        .data(store)
        .finish()
}

#[derive(SimpleObject)]
pub struct BlockWithCanonicity {
    /// Value canonical
    canonical: bool,
    /// Value block
    #[graphql(flatten)]
    block: Block,
}

#[derive(SimpleObject)]
struct Block {
    /// Value state_hash
    state_hash: String,
    /// Value block_height
    block_height: u32,
    /// Value winning_account
    winner_account: WinnerAccount,
    /// Value date_time as ISO 8601 string
    date_time: String,
    // Value received_time as ISO 8601 string
    received_time: String,
    /// Value creator account
    creator_account: CreatorAccount,
    // Value creator public key
    creator: String,
    // Value protocol state
    protocol_state: ProtocolState,
    // Value transaction fees
    tx_fees: String
}

#[derive(SimpleObject)]
struct ProtocolState {
    // Value parent state hash
    previous_state_hash: String,
}

#[derive(SimpleObject)]
struct WinnerAccount {
    /// The public_key for the WinnerAccount
    public_key: String,
}

#[derive(SimpleObject)]
struct CreatorAccount {
    /// The public_key for the WinnerAccount
    public_key: String,
}

/// convert epoch millis to an ISO 8601 formatted date
fn millis_to_date_string(millis: i64) -> String {
    let date_time = DateTime::from_timestamp_millis(millis).unwrap();
    date_time.to_rfc3339_opts(SecondsFormat::Millis, true)
}

impl From<PrecomputedBlock> for Block {
    fn from(block: PrecomputedBlock) -> Self {
        let winner_account = block.block_creator().0;
        let date_time = millis_to_date_string(block.timestamp().try_into().unwrap());
        let pk_creator = block.consensus_state().block_creator;
        let creator = CompressedPubKey::from(&pk_creator).into_address();
        let scheduled_time = block.scheduled_time.clone();
        let received_time = millis_to_date_string(scheduled_time.parse::<i64>().unwrap());
        let previous_state_hash = block.previous_state_hash().0;
        let tx_fees = block.tx_fees();
        Block {
            state_hash: block.state_hash,
            block_height: block.blockchain_length,
            date_time,
            winner_account: WinnerAccount {
                public_key: winner_account,
            },
            creator_account: CreatorAccount {
                public_key: creator.clone(),
            },
            creator,
            received_time,
            protocol_state: ProtocolState {
                previous_state_hash,
            },
            tx_fees: tx_fees.to_string(),
        }
    }
}
