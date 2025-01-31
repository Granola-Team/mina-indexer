use crate::{
    base::amount::Amount,
    block::{precomputed::PrecomputedBlock, store::BlockStore},
    chain::store::ChainStore,
    command::{internal::store::InternalCommandStore, store::UserCommandStore},
    constants::{MAINNET_EPOCH_SLOT_COUNT, VERSION},
    ledger::store::best::BestLedgerStore,
    snark_work::store::SnarkStore,
    store::{
        version::{IndexerStoreVersion, VersionStore},
        IndexerStore,
    },
    utility::functions::nanomina_to_mina,
    web::rest::locked_balances::LockedBalances,
};
use actix_web::{get, http::header::ContentType, web::Data, HttpResponse};
use chrono::DateTime;
use log::trace;
use serde::Serialize;
use std::sync::Arc;

/// Returns blockchain summary information about the current chain
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct BlockchainSummary {
    blockchain_length: u32,
    chain_id: String,
    circulating_supply: String,
    date_time: String,
    epoch: u32,
    slot: u32,
    global_slot: u32,
    locked_supply: String,
    min_window_density: u32,
    next_epoch_ledger_hash: String,
    previous_state_hash: String,
    snarked_ledger_hash: Option<String>,
    staged_ledger_hash: String,
    staking_epoch_ledger_hash: String,
    state_hash: String,
    total_currency: String,
    total_num_accounts: u32,
    epoch_num_blocks: u32,
    total_num_blocks: u32,
    epoch_num_snarks: u32,
    total_num_snarks: u32,
    total_num_canonical_snarks: u32,
    epoch_num_user_commands: u32,
    total_num_user_commands: u32,
    total_num_applied_user_commands: u32,
    total_num_failed_user_commands: u32,
    total_num_canonical_user_commands: u32,
    total_num_applied_canonical_user_commands: u32,
    total_num_failed_canonical_user_commands: u32,
    epoch_num_internal_commands: u32,
    total_num_internal_commands: u32,
    total_num_canonical_internal_commands: u32,
    db_version: String,
    indexer_version: String,
}

fn millis_to_date_string(millis: i64) -> String {
    let date_time = DateTime::from_timestamp_millis(millis).unwrap();
    // RFC 2822 date format
    date_time.format("%a, %d %b %Y %H:%M:%S GMT").to_string()
}

struct SummaryInput {
    chain_id: String,
    best_tip: PrecomputedBlock,
    locked_balance: Option<Amount>,
    db_version: IndexerStoreVersion,
    indexer_version: String,
    epoch_num_blocks: u32,
    total_num_blocks: u32,
    epoch_num_snarks: u32,
    total_num_snarks: u32,
    total_num_canonical_snarks: u32,
    epoch_num_user_commands: u32,
    total_num_user_commands: u32,
    total_num_applied_user_commands: u32,
    total_num_failed_user_commands: u32,
    total_num_canonical_user_commands: u32,
    total_num_applied_canonical_user_commands: u32,
    total_num_failed_canonical_user_commands: u32,
    epoch_num_internal_commands: u32,
    total_num_internal_commands: u32,
    total_num_canonical_internal_commands: u32,
    total_num_accounts: u32,
}

fn calculate_summary(input: SummaryInput) -> Option<BlockchainSummary> {
    let SummaryInput {
        chain_id,
        best_tip,
        locked_balance,
        db_version,
        indexer_version,
        epoch_num_blocks,
        total_num_blocks,
        epoch_num_snarks,
        total_num_snarks,
        total_num_canonical_snarks,
        epoch_num_user_commands,
        total_num_user_commands,
        total_num_applied_user_commands,
        total_num_failed_user_commands,
        total_num_canonical_user_commands,
        total_num_applied_canonical_user_commands,
        total_num_failed_canonical_user_commands,
        epoch_num_internal_commands,
        total_num_internal_commands,
        total_num_canonical_internal_commands,
        total_num_accounts,
    } = input;
    let blockchain_length = best_tip.blockchain_length();
    let date_time = millis_to_date_string(best_tip.timestamp() as i64);
    let epoch = best_tip.epoch_count();
    let global_slot = best_tip.global_slot_since_genesis();
    let min_window_density = best_tip.min_window_density();
    let next_epoch_ledger_hash = best_tip.next_epoch_ledger_hash().0;
    let previous_state_hash = best_tip.previous_state_hash().0;
    let slot = global_slot % MAINNET_EPOCH_SLOT_COUNT;
    let snarked_ledger_hash = best_tip.snarked_ledger_hash().map(|hash| hash.0);
    let staged_ledger_hash = best_tip.staged_ledger_hash().0;
    let staking_epoch_ledger_hash = best_tip.staking_epoch_ledger_hash().0;
    let state_hash = best_tip.state_hash().0;
    let total_currency_u64 = best_tip.total_currency();
    let locked_currency_u64 = locked_balance.map(|a| a.0).unwrap_or_default();
    let total_currency = nanomina_to_mina(total_currency_u64);
    let circulating_supply = nanomina_to_mina(total_currency_u64 - locked_currency_u64);
    let locked_supply = nanomina_to_mina(locked_currency_u64);
    let db_version = db_version.to_string();

    Some(BlockchainSummary {
        blockchain_length,
        chain_id,
        circulating_supply,
        date_time,
        epoch,
        global_slot,
        locked_supply,
        min_window_density,
        next_epoch_ledger_hash,
        previous_state_hash,
        slot,
        snarked_ledger_hash,
        staged_ledger_hash,
        staking_epoch_ledger_hash,
        state_hash,
        total_currency,
        total_num_accounts,
        epoch_num_blocks,
        total_num_blocks,
        epoch_num_snarks,
        total_num_snarks,
        total_num_canonical_snarks,
        epoch_num_user_commands,
        total_num_user_commands,
        total_num_applied_user_commands,
        total_num_failed_user_commands,
        total_num_canonical_user_commands,
        total_num_applied_canonical_user_commands,
        total_num_failed_canonical_user_commands,
        epoch_num_internal_commands,
        total_num_internal_commands,
        total_num_canonical_internal_commands,
        db_version,
        indexer_version,
    })
}

#[get("/summary")]
pub async fn get_blockchain_summary(
    store: Data<Arc<IndexerStore>>,
    locked_balances: Data<Arc<LockedBalances>>,
) -> HttpResponse {
    let db = store.as_ref();
    if let Ok(Some(best_tip)) = db.get_best_block() {
        trace!("Found best tip: {}", best_tip.summary());
        let total_num_accounts = store
            .get_num_accounts()
            .expect("num accounts")
            .unwrap_or_default();

        // aggregated on-chain & off-chain time-locked tokens
        let chain_id = store.get_chain_id().expect("chain id").0;
        let global_slot = best_tip.global_slot_since_genesis();
        let locked_balance = locked_balances.get_locked_amount(global_slot);

        // version info
        let db_version = store.get_db_version().expect("store version");
        let indexer_version = VERSION.to_string();

        // epoch & total data counts
        let epoch_num_blocks = store
            .get_block_production_epoch_count(None)
            .expect("epoch blocks count");
        let total_num_blocks = store
            .get_block_production_total_count()
            .expect("total blocks count");
        let epoch_num_snarks = store
            .get_snarks_epoch_count(None)
            .expect("epoch snarks count");
        let total_num_snarks = store.get_snarks_total_count().expect("total snarks count");
        let total_num_canonical_snarks = store
            .get_snarks_total_canonical_count()
            .expect("total canonical snarks count");
        let epoch_num_user_commands = store
            .get_user_commands_epoch_count(None)
            .expect("epoch user commands count");
        let total_num_user_commands = store
            .get_user_commands_total_count()
            .expect("total user commands count");
        let total_num_applied_user_commands = store
            .get_applied_user_commands_count()
            .expect("total applied user commands count");
        let total_num_canonical_user_commands = store
            .get_canonical_user_commands_count()
            .expect("total canonical user commands count");
        let total_num_applied_canonical_user_commands = store
            .get_applied_canonical_user_commands_count()
            .expect("total applied canonical user commands count");
        let total_num_failed_canonical_user_commands = store
            .get_failed_canonical_user_commands_count()
            .expect("total failed canonical user commands count");
        let total_num_failed_user_commands = store
            .get_failed_user_commands_count()
            .expect("total failed user commands count");
        let epoch_num_internal_commands = store
            .get_internal_commands_epoch_count(None)
            .expect("epoch internal commands count");
        let total_num_internal_commands = store
            .get_internal_commands_total_count()
            .expect("total internal commands count");
        let total_num_canonical_internal_commands = store
            .get_canonical_internal_commands_count()
            .expect("total number of canonical internal commands");

        if let Some(ref summary) = calculate_summary(SummaryInput {
            chain_id,
            best_tip,
            locked_balance,
            db_version,
            indexer_version,
            epoch_num_blocks,
            total_num_blocks,
            epoch_num_snarks,
            total_num_snarks,
            total_num_canonical_snarks,
            epoch_num_user_commands,
            total_num_user_commands,
            total_num_applied_user_commands,
            total_num_failed_user_commands,
            total_num_canonical_user_commands,
            total_num_applied_canonical_user_commands,
            total_num_failed_canonical_user_commands,
            epoch_num_internal_commands,
            total_num_internal_commands,
            total_num_canonical_internal_commands,
            total_num_accounts,
        }) {
            trace!("Blockchain summary: {summary:?}");
            let body = serde_json::to_string_pretty(summary).expect("blockchain summary");
            return HttpResponse::Ok()
                .content_type(ContentType::json())
                .body(body);
        }
    }
    HttpResponse::NotFound().finish()
}
