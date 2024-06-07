use crate::{
    block::{precomputed::PrecomputedBlock, store::BlockStore},
    chain::store::ChainStore,
    command::{internal::store::InternalCommandStore, store::UserCommandStore},
    ledger::{
        account::{nanomina_to_mina, Amount},
        store::LedgerStore,
        LedgerHash,
    },
    snark_work::store::SnarkStore,
    store::IndexerStore,
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
    global_slot: u32,
    locked_supply: String,
    min_window_density: u32,
    next_epoch_ledger_hash: String,
    previous_state_hash: String,
    slot: u32,
    snarked_ledger_hash: String,
    staged_ledger_hash: String,
    staking_epoch_ledger_hash: String,
    state_hash: String,
    total_currency: String,
    total_num_accounts: usize,
    epoch_num_blocks: u32,
    total_num_blocks: u32,
    epoch_num_snarks: u32,
    total_num_snarks: u32,
    epoch_num_user_commands: u32,
    total_num_user_commands: u32,
    epoch_num_internal_commands: u32,
    total_num_internal_commands: u32,
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
    epoch_num_blocks: u32,
    total_num_blocks: u32,
    epoch_num_snarks: u32,
    total_num_snarks: u32,
    epoch_num_user_commands: u32,
    total_num_user_commands: u32,
    epoch_num_internal_commands: u32,
    total_num_internal_commands: u32,
    total_num_accounts: usize,
}

fn calculate_summary(input: SummaryInput) -> Option<BlockchainSummary> {
    let SummaryInput {
        chain_id,
        best_tip,
        locked_balance,
        epoch_num_blocks,
        total_num_blocks,
        epoch_num_snarks,
        total_num_snarks,
        epoch_num_user_commands,
        total_num_user_commands,
        epoch_num_internal_commands,
        total_num_internal_commands,
        total_num_accounts,
    } = input;
    let blockchain_length = best_tip.blockchain_length();
    let date_time = millis_to_date_string(best_tip.timestamp().try_into().unwrap());
    let epoch = best_tip.epoch_count();
    let global_slot = best_tip.global_slot_since_genesis();
    let min_window_density = best_tip.consensus_state().min_window_density.t.t;
    let next_epoch_ledger_hash = LedgerHash::from_hashv1(
        best_tip
            .consensus_state()
            .next_epoch_data
            .inner()
            .inner()
            .ledger
            .inner()
            .inner()
            .hash,
    )
    .0;

    let previous_state_hash = best_tip.previous_state_hash().0;
    let slot = global_slot - (epoch * 7140);
    let snarked_ledger_hash =
        LedgerHash::from_hashv1(best_tip.blockchain_state().snarked_ledger_hash).0;
    let staged_ledger_hash = LedgerHash::from_hashv1(
        best_tip
            .blockchain_state()
            .staged_ledger_hash
            .inner()
            .inner()
            .non_snark
            .inner()
            .ledger_hash,
    )
    .0;
    let staking_epoch_ledger_hash = LedgerHash::from_hashv1(
        best_tip
            .consensus_state()
            .staking_epoch_data
            .inner()
            .inner()
            .ledger
            .inner()
            .inner()
            .hash,
    )
    .0;
    let state_hash = best_tip.state_hash().0;
    let total_currency_u64 = best_tip.consensus_state().total_currency.inner().inner();
    let locked_currency_u64 = locked_balance.map(|a| a.0).unwrap_or(0);
    let total_currency = nanomina_to_mina(total_currency_u64);
    let circulating_supply = nanomina_to_mina(total_currency_u64 - locked_currency_u64);
    let locked_supply = nanomina_to_mina(locked_currency_u64);

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
        epoch_num_user_commands,
        total_num_user_commands,
        epoch_num_internal_commands,
        total_num_internal_commands,
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
        let state_hash = &best_tip.state_hash();
        let total_num_accounts = store
            .get_ledger_state_hash(state_hash, true)
            .expect("ledger exists")
            .map(|ledger| ledger.len())
            .unwrap_or(0_usize);

        // aggregated on-chain & off-chain time-locked tokens
        let chain_id = store.get_chain_id().expect("chain id").0;
        let global_slot = best_tip.global_slot_since_genesis();
        let locked_balance = locked_balances.get_locked_amount(global_slot);

        // get epoch & total info
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
        let epoch_num_user_commands = store
            .get_user_commands_epoch_count(None)
            .expect("epoch user commands count");
        let total_num_user_commands = store
            .get_user_commands_total_count()
            .expect("total user commands count");
        let epoch_num_internal_commands = store
            .get_internal_commands_epoch_count(None)
            .expect("epoch internal commands count");
        let total_num_internal_commands = store
            .get_internal_commands_total_count()
            .expect("total internal commands count");

        if let Some(ref summary) = calculate_summary(SummaryInput {
            chain_id,
            best_tip,
            locked_balance,
            epoch_num_blocks,
            total_num_blocks,
            epoch_num_snarks,
            total_num_snarks,
            epoch_num_user_commands,
            total_num_user_commands,
            epoch_num_internal_commands,
            total_num_internal_commands,
            total_num_accounts,
        }) {
            trace!("Blockchain summary: {:?}", summary);
            let body = serde_json::to_string_pretty(summary).expect("blockchain summary");
            return HttpResponse::Ok()
                .content_type(ContentType::json())
                .body(body);
        }
    }
    HttpResponse::NotFound().finish()
}
