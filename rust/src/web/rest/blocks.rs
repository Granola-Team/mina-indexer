use crate::{
    block::{
        is_valid_state_hash,
        precomputed::{PrecomputedBlock, PrecomputedBlockWithCanonicity},
        store::BlockStore,
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
    height: Option<u32>,
}

fn get_limit(limit: Option<u32>) -> u32 {
    limit.map(|value| value.min(100)).unwrap_or(10)
}

fn format_blocks(blocks: Vec<PrecomputedBlockWithCanonicity>) -> String {
    format!("{blocks:#?}").replace(",\n]", "\n]")
}

#[get("/blocks")]
pub async fn get_blocks(
    store: Data<Arc<IndexerStore>>,
    params: web::Query<Params>,
) -> HttpResponse {
    let db = store.as_ref();
    let limit = get_limit(params.limit);

    // Check for height query parameter
    if let Some(height) = params.height {
        if let Ok(blocks) = db.get_blocks_at_height(height) {
            let blocks: Vec<PrecomputedBlockWithCanonicity> = blocks
                .iter()
                .flat_map(|state_hash| {
                    if let Ok(Some(canonicity)) = db.get_block_canonicity(state_hash) {
                        let block = db
                            .get_block(state_hash)
                            .with_context(|| format!("block missing from store {state_hash}"))
                            .unwrap()
                            .unwrap()
                            .0;
                        Some(PrecomputedBlock::with_canonicity(&block, canonicity))
                    } else {
                        None
                    }
                })
                .take(limit as usize)
                .collect();
            return HttpResponse::Ok()
                .content_type(ContentType::json())
                .body(format_blocks(blocks));
        }
    }

    if let Ok(Some(best_tip)) = db.get_best_block() {
        let mut best_chain: Vec<PrecomputedBlockWithCanonicity> =
            Vec::with_capacity(limit as usize);

        // Process best tip
        if let Ok(Some(canonicity)) = db.get_block_canonicity(&best_tip.state_hash()) {
            best_chain.push(PrecomputedBlock::with_canonicity(&best_tip, canonicity));
        }

        let mut parent_state_hash = best_tip.previous_state_hash();

        while best_chain.len() < limit as usize {
            if let Ok(Some((block, _))) = db.get_block(&parent_state_hash) {
                if let Ok(Some(canonicity)) = db.get_block_canonicity(&block.state_hash()) {
                    best_chain.push(PrecomputedBlock::with_canonicity(&block, canonicity));
                    parent_state_hash = block.previous_state_hash();
                } else {
                    break;
                }
            } else {
                // No parent
                break;
            }
        }

        return HttpResponse::Ok()
            .content_type(ContentType::json())
            .body(format_blocks(best_chain));
    }
    HttpResponse::NotFound().finish()
}

#[get("/blocks/{state_hash}")]
pub async fn get_block(
    store: Data<Arc<IndexerStore>>,
    state_hash: web::Path<String>,
) -> HttpResponse {
    let db = store.as_ref();

    if is_valid_state_hash(&state_hash) {
        if let Ok(Some((ref block, _))) = db.get_block(&state_hash.clone().into()) {
            if let Ok(Some(canonicity)) = db.get_block_canonicity(&block.state_hash()) {
                let block_with_canonicity = PrecomputedBlock::with_canonicity(block, canonicity);
                return HttpResponse::Ok()
                    .content_type(ContentType::json())
                    .body(format!("{block_with_canonicity:?}"));
            }
        }
    }

    HttpResponse::NotFound().finish()
}
