use crate::{
    block::{precomputed::PrecomputedBlock, store::BlockStore},
    store::IndexerStore,
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
}

fn get_limit(limit: Option<u32>) -> u32 {
    limit.map(|value| value.min(10)).unwrap_or(1)
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
        let mut counter = 1;
        let mut parent_state_hash = best_tip.previous_state_hash();

        loop {
            if counter == limit {
                break;
            }
            if let Ok(Some(block)) = db.get_block(&parent_state_hash) {
                parent_state_hash = block.previous_state_hash();
                best_chain.push(block);
            } else {
                // No parent
                break;
            }
            counter += 1;
        }
        let body = serde_json::to_string(&best_chain).unwrap();
        return HttpResponse::Ok()
            .content_type(ContentType::json())
            .body(body);
    }
    HttpResponse::NotFound().finish()
}

#[get("/blocks/{state_hash}")]
pub async fn get_block(
    store: Data<Arc<IndexerStore>>,
    state_hash: web::Path<String>,
) -> HttpResponse {
    let db = store.as_ref();
    if let Ok(Some(ref block)) = db.get_block(&state_hash.clone().into()) {
        let body = serde_json::to_string(block).unwrap();
        return HttpResponse::Ok()
            .content_type(ContentType::json())
            .body(body);
    }
    HttpResponse::NotFound().finish()
}
