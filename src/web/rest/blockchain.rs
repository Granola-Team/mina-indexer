use std::sync::Arc;

use crate::protocol::serialization_types::{
    common::{Base58EncodableVersionedType, HashV1},
    version_bytes,
};
use actix_web::{get, http::header::ContentType, web::Data, HttpResponse};
use chrono::DateTime;
use serde::Serialize;
use tracing::debug;

use crate::{
    block::{precomputed::PrecomputedBlock, store::BlockStore},
    ledger::{
        account::{nanomina_to_mina, Amount},
        store::LedgerStore,
        Ledger,
    },
    store::IndexerStore,
    web::rest::locked_balances::LockedBalances,
};

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
}

pub struct LedgerHash(pub String);

impl LedgerHash {
    pub fn from_hashv1(hashv1: HashV1) -> Self {
        let versioned: Base58EncodableVersionedType<{ version_bytes::LEDGER_HASH }, _> =
            hashv1.into();
        Self(versioned.to_base58_string().unwrap())
    }
}

fn millis_to_date_string(millis: i64) -> String {
    let date_time = DateTime::from_timestamp_millis(millis).unwrap();
    // RFC 2822 date format
    date_time.format("%a, %d %b %Y %H:%M:%S GMT").to_string()
}

fn calculate_summary(
    best_tip: &PrecomputedBlock,
    _best_ledger: &Ledger,
    locked_balance: Option<Amount>,
) -> Option<BlockchainSummary> {
    let blockchain_length = best_tip.blockchain_length;
    let chain_id = "5f704cc0c82e0ed70e873f0893d7e06f148524e3f0bdae2afb02e7819a0c24d1".to_owned();
    let date_time = millis_to_date_string(best_tip.timestamp().try_into().unwrap());
    let epoch = best_tip.consensus_state().epoch_count.inner().inner();
    let global_slot = best_tip
        .consensus_state()
        .global_slot_since_genesis
        .inner()
        .inner();
    let min_window_density = best_tip
        .consensus_state()
        .min_window_density
        .inner()
        .inner();
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
    let slot = best_tip
        .consensus_state()
        .curr_global_slot
        .inner()
        .inner()
        .slot_number
        .inner()
        .inner();
    let snarked_ledger_hash = LedgerHash::from_hashv1(
        best_tip
            .protocol_state
            .body
            .clone()
            .inner()
            .inner()
            .blockchain_state
            .inner()
            .inner()
            .snarked_ledger_hash,
    )
        .0;
    let staged_ledger_hash = LedgerHash::from_hashv1(
        best_tip
            .protocol_state
            .body
            .clone()
            .inner()
            .inner()
            .blockchain_state
            .inner()
            .inner()
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
    let state_hash = best_tip.state_hash.clone();
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
    })
}

#[get("/summary")]
pub async fn get_blockchain_summary(
    store: Data<Arc<IndexerStore>>,
    locked_balances: Data<Arc<LockedBalances>>,
) -> HttpResponse {
    let db = store.as_ref();
    if let Ok(Some(best_tip)) = db.get_best_block() {
        debug!("Found best tip: {:?}", best_tip.state_hash);
        if let Ok(Some(best_ledger)) = db.get_ledger_state_hash(&best_tip.state_hash.clone().into())
        {
            let global_slot = best_tip
                .consensus_state()
                .global_slot_since_genesis
                .inner()
                .inner();
            let locked_amount = locked_balances.get_locked_amount(global_slot);
            debug!("Found ledger for best tip");
            if let Some(ref summary) = calculate_summary(&best_tip, &best_ledger, locked_amount) {
                debug!("Blockchain summary for the best_tip: {:?}", summary);
                let body = serde_json::to_string_pretty(summary).unwrap();
                return HttpResponse::Ok()
                    .content_type(ContentType::json())
                    .body(body);
            }
        }
    }
    HttpResponse::NotFound().finish()
}
