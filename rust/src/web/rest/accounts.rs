use crate::{block::store::BlockStore, ledger::store::LedgerStore, store::IndexerStore};
use actix_web::{
    get,
    http::header::ContentType,
    web::{self, Data},
    HttpResponse,
};
use log::debug;
use std::sync::Arc;

#[get("/accounts/{public_key}")]
pub async fn get_account(
    store: Data<Arc<IndexerStore>>,
    public_key: web::Path<String>,
) -> HttpResponse {
    let db = store.as_ref();
    if let Ok(Some(best_tip)) = db.get_best_block() {
        debug!("Found best tip {}", best_tip.summary());
        if let Ok(Some(ledger)) = db.get_ledger_state_hash(&best_tip.state_hash(), true) {
            debug!("Found ledger for best tip");
            let account = ledger.accounts.get(&public_key.clone().into());
            if let Some(ref account) = account {
                debug!("Found account in ledger: {:?}", account);
                let body = serde_json::to_string(account).unwrap();
                return HttpResponse::Ok()
                    .content_type(ContentType::json())
                    .body(body);
            }
        }
    }
    HttpResponse::NotFound().finish()
}
