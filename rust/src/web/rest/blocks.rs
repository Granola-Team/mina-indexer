use crate::{
    block::{is_valid_state_hash, store::BlockStore},
    store::IndexerStore,
    web::graphql::{
        blocks::{get_counts, Block},
        get_block,
    },
};
use actix_web::{
    get,
    http::header::ContentType,
    web::{self, Data},
    HttpResponse,
};
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

fn format_blocks(blocks: Vec<Block>) -> String {
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
            let counts = get_counts(db).await.expect("counts");

            let blocks = blocks
                .iter()
                .flat_map(|state_hash| {
                    let block = get_block(db, state_hash);
                    Some(Block::from_precomputed(db, &block, counts))
                })
                .take(limit as usize)
                .collect();
            return HttpResponse::Ok()
                .content_type(ContentType::json())
                .body(format_blocks(blocks));
        }
    }

    if let Ok(Some(best_tip)) = db.get_best_block() {
        let mut best_chain: Vec<Block> = Vec::with_capacity(limit as usize);

        // Process best tip
        best_chain.push(Block::from_precomputed(
            db,
            &best_tip,
            get_counts(db).await.expect("counts"),
        ));

        let mut parent_state_hash = best_tip.previous_state_hash();

        while best_chain.len() < limit as usize {
            if let Ok(Some((block, _))) = db.get_block(&parent_state_hash) {
                best_chain.push(Block::from_precomputed(
                    db,
                    &block,
                    get_counts(db).await.expect("counts"),
                ));
                parent_state_hash = block.previous_state_hash();
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
pub async fn get_block_by_state_hash(
    store: Data<Arc<IndexerStore>>,
    state_hash: web::Path<String>,
) -> HttpResponse {
    let db = store.as_ref();

    if is_valid_state_hash(&state_hash) {
        if let Ok(Some((ref block, _))) = db.get_block(&state_hash.clone().into()) {
            let block = Block::from_precomputed(db, block, get_counts(db).await.expect("counts"));
            return HttpResponse::Ok()
                .content_type(ContentType::json())
                .body(format!("{block:?}"));
        }
    }

    HttpResponse::NotFound().finish()
}
