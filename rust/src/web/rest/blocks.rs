use crate::{
    block::{
        is_valid_state_hash, precomputed::PrecomputedBlock, store::BlockStore, BlockWithoutHeight,
    },
    canonicity::store::CanonicityStore,
    store::IndexerStore,
};
use actix_web::{
    get,
    http::header::ContentType,
    web::{self, Data},
    HttpResponse,
};
use anyhow::Context as aContext;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
struct Params {
    limit: Option<u32>,
}

fn get_limit(limit: Option<u32>) -> u32 {
    limit.map(|value| value.min(100)).unwrap_or(10)
}

fn format_blocks(blocks: Vec<BlockWithoutHeight>) -> String {
    format!("{blocks:#?}").replace(",\n]", "\n]")
}

#[get("/blocks")]
pub async fn get_blocks(
    store: Data<Arc<IndexerStore>>,
    params: web::Query<Params>,
) -> HttpResponse {
    let db = store.as_ref();
    let limit = get_limit(params.limit);

    if let Ok(Some(best_tip)) = db.get_best_block() {
        let mut best_chain: Box<Vec<PrecomputedBlock>> = Box::new(vec![best_tip.clone()]);
        let mut parent_state_hash = best_tip.previous_state_hash();

        loop {
            if best_chain.len() == limit as usize {
                break;
            }

            if let Ok(Some(block)) = db.get_block(&parent_state_hash) {
                parent_state_hash = block.previous_state_hash();
                best_chain.push(block);
            } else {
                // No parent
                break;
            }
        }

        let best_chain: Vec<BlockWithoutHeight> = best_chain
            .iter()
            .flat_map(|block| {
                if let Ok(Some(canonicity)) = db.get_block_canonicity(&block.state_hash()) {
                    Some(BlockWithoutHeight::with_canonicity(block, canonicity))
                } else {
                    None
                }
            })
            .collect();
        return HttpResponse::Ok()
            .content_type(ContentType::json())
            .body(format_blocks(best_chain));
    }
    HttpResponse::NotFound().finish()
}

#[get("/blocks/{input}")]
pub async fn get_block(store: Data<Arc<IndexerStore>>, input: web::Path<String>) -> HttpResponse {
    let db = store.as_ref();

    // via state hash
    if is_valid_state_hash(&input) {
        if let Ok(Some(ref block)) = db.get_block(&input.clone().into()) {
            let block: BlockWithoutHeight = block.into();
            return HttpResponse::Ok()
                .content_type(ContentType::json())
                .body(format!("{block:?}"));
        }
    }

    // via blockchain length
    let height_prefix = "height=";
    if (*input).starts_with(height_prefix) {
        if let Ok(height) = input[height_prefix.len()..].parse::<u32>() {
            if let Ok(blocks) = db.get_blocks_at_height(height) {
                let blocks: Vec<BlockWithoutHeight> = blocks
                    .iter()
                    .flat_map(|state_hash| {
                        if let Ok(Some(canonicity)) = db.get_block_canonicity(state_hash) {
                            let block = db
                                .get_block(&state_hash)
                                .with_context(|| format!("block missing from store {state_hash}"))
                                .unwrap()
                                .unwrap();
                            Some(BlockWithoutHeight::with_canonicity(&block, canonicity))
                        } else {
                            None
                        }
                    })
                    .collect();
                return HttpResponse::Ok()
                    .content_type(ContentType::json())
                    .body(format_blocks(blocks));
            }
        }
    }

    // via global slot
    let slot_prefix = "slot=";
    if (*input).starts_with(slot_prefix) {
        if let Ok(slot) = input[slot_prefix.len()..].parse::<u32>() {
            if let Ok(blocks) = db.get_blocks_at_slot(slot) {
                let blocks: Vec<BlockWithoutHeight> = blocks
                    .iter()
                    .flat_map(|state_hash| {
                        if let Ok(Some(canonicity)) = db.get_block_canonicity(state_hash) {
                            let block = db
                                .get_block(&state_hash)
                                .with_context(|| format!("block missing from store {state_hash}"))
                                .unwrap()
                                .unwrap();
                            Some(BlockWithoutHeight::with_canonicity(&block, canonicity))
                        } else {
                            None
                        }
                    })
                    .collect();
                return HttpResponse::Ok()
                    .content_type(ContentType::json())
                    .body(format_blocks(blocks));
            }
        }
    }

    HttpResponse::NotFound().finish()
}
