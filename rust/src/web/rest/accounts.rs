use crate::{
    block::store::BlockStore,
    command::{internal::store::InternalCommandStore, store::UserCommandStore},
    constants::MAINNET_ACCOUNT_CREATION_FEE,
    ledger::{account, public_key::PublicKey, store::best::BestLedgerStore},
    snark_work::store::SnarkStore,
    store::IndexerStore,
};
use actix_web::{
    get,
    http::header::ContentType,
    web::{self, Data},
    HttpResponse,
};
use log::debug;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Account {
    #[serde(flatten)]
    account: account::Account,
    epoch_num_blocks: u32,
    total_num_blocks: u32,
    epoch_num_snarks: u32,
    total_num_snarks: u32,
    epoch_num_user_commands: u32,
    total_num_user_commands: u32,
    epoch_num_internal_commands: u32,
    total_num_internal_commands: u32,
}

#[get("/accounts/{public_key}")]
pub async fn get_account(
    store: Data<Arc<IndexerStore>>,
    public_key: web::Path<String>,
) -> HttpResponse {
    let db = store.as_ref();
    let pk: PublicKey = public_key.clone().into();
    if let Ok(Some(account)) = db.get_best_account(&pk) {
        debug!("Found account in ledger: {account}");
        let account = Account {
            account: account.clone(),
            epoch_num_blocks: db
                .get_block_production_pk_epoch_count(&pk, None)
                .unwrap_or_default(),
            total_num_blocks: db
                .get_block_production_pk_total_count(&pk)
                .unwrap_or_default(),
            epoch_num_snarks: db.get_snarks_pk_epoch_count(&pk, None).unwrap_or_default(),
            total_num_snarks: db.get_snarks_pk_total_count(&pk).unwrap_or_default(),
            epoch_num_user_commands: db
                .get_user_commands_pk_epoch_count(&pk, None)
                .unwrap_or_default(),
            total_num_user_commands: db.get_user_commands_pk_total_count(&pk).unwrap_or_default(),
            epoch_num_internal_commands: db
                .get_internal_commands_pk_epoch_count(&pk, None)
                .unwrap_or_default(),
            total_num_internal_commands: db
                .get_internal_commands_pk_total_count(&pk)
                .unwrap_or_default(),
        };
        return HttpResponse::Ok().content_type(ContentType::json()).body(
            serde_json::to_string_pretty(&Account {
                account: account::Account {
                    // deduct fee for display
                    balance: account.account.balance - MAINNET_ACCOUNT_CREATION_FEE,
                    ..account.account.clone()
                },
                ..account
            })
            .expect("serde account bytes"),
        );
    }
    HttpResponse::NotFound().finish()
}
