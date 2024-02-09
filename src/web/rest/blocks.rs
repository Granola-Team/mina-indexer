use std::sync::Arc;

use actix_web::{
    get,
    http::header::ContentType,
    web::{self, Data},
    HttpResponse,
};

use crate::{block::store::BlockStore, store::IndexerStore};

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
