use crate::{
    base::public_key::PublicKey,
    block::store::BlockStore,
    command::{internal::store::InternalCommandStore, store::UserCommandStore},
    ledger::{account, store::best::BestLedgerStore, token::TokenAddress},
    snark_work::store::SnarkStore,
    store::{username::UsernameStore, IndexerStore},
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

    // accounts
    total_num_accounts: u32,
    total_num_zkapp_accounts: u32,

    // blocks
    epoch_num_blocks: u32,
    total_num_blocks: u32,

    // SNARKs
    epoch_num_snarks: u32,
    total_num_snarks: u32,

    // all user commands
    epoch_num_user_commands: u32,
    total_num_user_commands: u32,

    // zkapp user commands
    epoch_num_zkapp_commands: u32,
    total_num_zkapp_commands: u32,

    // internal commands
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

    if let Ok(Some(account)) = db.get_best_account(&pk, &TokenAddress::default()) {
        debug!("Found account in ledger: {account}");

        let account = Account {
            account: account::Account {
                username: db.get_username(&pk).unwrap_or_default(),
                ..account
            },

            // accounts
            total_num_accounts: db
                .get_num_accounts()
                .expect("num accounts")
                .unwrap_or_default(),
            total_num_zkapp_accounts: db
                .get_num_zkapp_accounts()
                .expect("num zkapp accounts")
                .unwrap_or_default(),

            // blocks
            epoch_num_blocks: db
                .get_block_production_pk_epoch_count(&pk, None, None)
                .unwrap_or_default(),
            total_num_blocks: db
                .get_block_production_pk_total_count(&pk)
                .unwrap_or_default(),

            // SNARKs
            epoch_num_snarks: db
                .get_snarks_pk_epoch_count(&pk, None, None)
                .unwrap_or_default(),
            total_num_snarks: db.get_snarks_pk_total_count(&pk).unwrap_or_default(),

            // all user commands
            epoch_num_user_commands: db
                .get_user_commands_pk_epoch_count(&pk, None, None)
                .unwrap_or_default(),
            total_num_user_commands: db.get_user_commands_pk_total_count(&pk).unwrap_or_default(),

            // zkapp user commands
            epoch_num_zkapp_commands: db
                .get_zkapp_commands_pk_epoch_count(&pk, None, None)
                .unwrap_or_default(),
            total_num_zkapp_commands: db
                .get_zkapp_commands_pk_total_count(&pk)
                .unwrap_or_default(),

            // internal commands
            epoch_num_internal_commands: db
                .get_internal_commands_pk_epoch_count(&pk, None, None)
                .unwrap_or_default(),
            total_num_internal_commands: db
                .get_internal_commands_pk_total_count(&pk)
                .unwrap_or_default(),
        };

        return HttpResponse::Ok().content_type(ContentType::json()).body(
            serde_json::to_string_pretty(&Account {
                account: account.account.clone().deduct_mina_account_creation_fee(),
                ..account
            })
            .expect("serde account bytes"),
        );
    }

    HttpResponse::NotFound().finish()
}
