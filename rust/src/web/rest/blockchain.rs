use crate::{
    base::{amount::Amount, state_hash::StateHash},
    block::{precomputed::PrecomputedBlock, store::BlockStore},
    chain::{store::ChainStore, ChainId},
    command::{internal::store::InternalCommandStore, store::UserCommandStore},
    constants::{MAINNET_EPOCH_SLOT_COUNT, VERSION},
    ledger::store::best::BestLedgerStore,
    snark_work::store::SnarkStore,
    store::{
        version::{IndexerStoreVersion, VersionStore},
        IndexerStore,
    },
    utility::functions::nanomina_to_mina,
    web::{common::unique_block_producers_last_n_blocks, rest::locked_balances::LockedBalances},
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
    chain_id: String,
    genesis_state_hash: String,

    blockchain_length: u32,
    date_time: String,
    epoch: u32,
    slot: u32,
    global_slot: u32,

    min_window_density: u32,

    // ledger hashes
    next_epoch_ledger_hash: String,
    snarked_ledger_hash: String,
    staged_ledger_hash: String,
    staking_epoch_ledger_hash: String,

    // self & parent hash
    state_hash: String,
    previous_state_hash: String,

    // currency
    total_currency: String,
    locked_supply: String,
    circulating_supply: String,

    // accounts
    total_num_accounts: u32,
    total_num_zkapp_accounts: u32,

    // blocks
    epoch_num_blocks: u32,
    total_num_blocks: u32,

    epoch_num_canonical_blocks: u32,

    num_unique_block_producers: Option<u32>,

    // SNARKs
    epoch_num_snarks: u32,
    total_num_snarks: u32,
    // epoch_num_canonical_snarks: u32,
    total_num_canonical_snarks: u32,

    // all user commands
    epoch_num_user_commands: u32,
    total_num_user_commands: u32,

    // applied user commands
    // epoch_num_applied_user_commands: u32,
    total_num_applied_user_commands: u32,

    // failed user commands
    // epoch_num_failed_user_commands: u32,
    total_num_failed_user_commands: u32,

    // canonical user commands
    // epoch_num_canonical_user_commands: u32,
    total_num_canonical_user_commands: u32,

    // applied canonical user commands
    // epoch_num_applied_canonical_user_commands: u32,
    total_num_applied_canonical_user_commands: u32,

    // failed canonical user commands
    // epoch_num_failed_canonical_user_commands: u32,
    total_num_failed_canonical_user_commands: u32,

    // zkapp user commands
    epoch_num_zkapp_commands: u32,
    total_num_zkapp_commands: u32,

    // applied zkapp commands
    // epoch_num_applied_zkapp_commands: u32,
    total_num_applied_zkapp_commands: u32,

    // failed zkapp commands
    // epoch_num_failed_zkapp_commands: u32,
    total_num_failed_zkapp_commands: u32,

    // canonical zkapp commands
    // epoch_num_canonical_zkapp_commands: u32,
    total_num_canonical_zkapp_commands: u32,

    // applied canonical zkapp commands
    // epoch_num_applied_canonical_zkapp_commands: u32,
    total_num_applied_canonical_zkapp_commands: u32,

    // failed canonical zkapp commands
    // epoch_num_failed_canonical_zkapp_commands: u32,
    total_num_failed_canonical_zkapp_commands: u32,

    // internal commands
    epoch_num_internal_commands: u32,
    total_num_internal_commands: u32,
    // epoch_num_canonical_internal_commands: u32,
    total_num_canonical_internal_commands: u32,

    // version
    db_version: String,
    indexer_version: String,
}

fn millis_to_date_string(millis: i64) -> String {
    let date_time = DateTime::from_timestamp_millis(millis).unwrap();
    // RFC 2822 date format
    date_time.format("%a, %d %b %Y %H:%M:%S GMT").to_string()
}

struct SummaryInput {
    chain_id: ChainId,
    genesis_state_hash: StateHash,

    best_tip: PrecomputedBlock,
    locked_balance: Option<Amount>,

    // accounts
    total_num_accounts: u32,
    total_num_zkapp_accounts: u32,

    // blocks
    epoch_num_blocks: u32,
    total_num_blocks: u32,

    epoch_num_canonical_blocks: u32,

    /// Unique block producer count in last n blocks
    num_unique_block_producers: Option<u32>,

    // SNARKs
    epoch_num_snarks: u32,
    total_num_snarks: u32,
    // epoch_num_canonical_snarks: u32,
    total_num_canonical_snarks: u32,

    // all user commands
    epoch_num_user_commands: u32,
    total_num_user_commands: u32,
    // epoch_num_applied_user_commands: u32,
    total_num_applied_user_commands: u32,
    // epoch_num_failed_user_commands: u32,
    total_num_failed_user_commands: u32,
    // epoch_num_canonical_user_commands: u32,
    total_num_canonical_user_commands: u32,
    // epoch_num_applied_canonical_user_commands: u32,
    total_num_applied_canonical_user_commands: u32,
    // epoch_num_failed_canonical_user_commands: u32,
    total_num_failed_canonical_user_commands: u32,

    // zkapp commands
    epoch_num_zkapp_commands: u32,
    total_num_zkapp_commands: u32,
    // epoch_num_applied_zkapp_commands: u32,
    total_num_applied_zkapp_commands: u32,
    // epoch_num_failed_zkapp_commands: u32,
    total_num_failed_zkapp_commands: u32,
    // epoch_num_canonical_zkapp_commands: u32,
    total_num_canonical_zkapp_commands: u32,
    // epoch_num_applied_canonical_zkapp_commands: u32,
    total_num_applied_canonical_zkapp_commands: u32,
    // epoch_num_failed_canonical_zkapp_commands: u32,
    total_num_failed_canonical_zkapp_commands: u32,

    // internal commands
    epoch_num_internal_commands: u32,
    total_num_internal_commands: u32,
    // epoch_num_canonical_internal_commands: u32,
    total_num_canonical_internal_commands: u32,

    // version
    db_version: IndexerStoreVersion,
    indexer_version: String,
}

impl BlockchainSummary {
    fn calculate_summary(input: SummaryInput) -> Option<Self> {
        let SummaryInput {
            chain_id,
            genesis_state_hash,

            best_tip,
            locked_balance,

            total_num_accounts,
            total_num_zkapp_accounts,

            epoch_num_blocks,
            total_num_blocks,
            epoch_num_canonical_blocks,
            num_unique_block_producers,

            epoch_num_snarks,
            total_num_snarks,
            total_num_canonical_snarks,

            epoch_num_user_commands,
            total_num_user_commands,
            total_num_canonical_user_commands,

            total_num_applied_user_commands,
            total_num_applied_canonical_user_commands,
            total_num_failed_user_commands,
            total_num_failed_canonical_user_commands,

            epoch_num_zkapp_commands,
            total_num_zkapp_commands,
            total_num_canonical_zkapp_commands,

            total_num_applied_zkapp_commands,
            total_num_applied_canonical_zkapp_commands,
            total_num_failed_zkapp_commands,
            total_num_failed_canonical_zkapp_commands,

            epoch_num_internal_commands,
            total_num_internal_commands,
            total_num_canonical_internal_commands,

            db_version,
            indexer_version,
        } = input;

        let chain_id = chain_id.to_string();
        let genesis_state_hash = genesis_state_hash.to_string();
        let blockchain_length = best_tip.blockchain_length();
        let date_time = millis_to_date_string(best_tip.timestamp() as i64);
        let epoch = best_tip.epoch_count();
        let global_slot = best_tip.global_slot_since_genesis();
        let min_window_density = best_tip.min_window_density();
        let next_epoch_ledger_hash = best_tip.next_epoch_ledger_hash().0;
        let previous_state_hash = best_tip.previous_state_hash().0;
        let slot = global_slot % MAINNET_EPOCH_SLOT_COUNT;
        let snarked_ledger_hash = best_tip.snarked_ledger_hash().0;
        let staged_ledger_hash = best_tip.staged_ledger_hash().0;
        let staking_epoch_ledger_hash = best_tip.staking_epoch_ledger_hash().0;
        let state_hash = best_tip.state_hash().0;
        let total_currency_u64 = best_tip.total_currency();
        let locked_currency_u64 = locked_balance.map(|a| a.0).unwrap_or_default();
        let total_currency = nanomina_to_mina(total_currency_u64);
        let circulating_supply = nanomina_to_mina(total_currency_u64 - locked_currency_u64);
        let locked_supply = nanomina_to_mina(locked_currency_u64);
        let db_version = db_version.to_string();

        Some(Self {
            chain_id,
            genesis_state_hash,

            date_time,
            epoch,
            blockchain_length,
            slot,
            global_slot,
            min_window_density,

            next_epoch_ledger_hash,
            snarked_ledger_hash,
            staged_ledger_hash,
            staking_epoch_ledger_hash,

            state_hash,
            previous_state_hash,

            total_currency,
            locked_supply,
            circulating_supply,

            total_num_accounts,
            total_num_zkapp_accounts,

            epoch_num_blocks,
            total_num_blocks,
            epoch_num_canonical_blocks,
            num_unique_block_producers,

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

            epoch_num_zkapp_commands,
            total_num_zkapp_commands,

            total_num_applied_zkapp_commands,
            total_num_failed_zkapp_commands,
            total_num_canonical_zkapp_commands,
            total_num_applied_canonical_zkapp_commands,
            total_num_failed_canonical_zkapp_commands,

            epoch_num_internal_commands,
            total_num_internal_commands,
            total_num_canonical_internal_commands,

            db_version,
            indexer_version,
        })
    }
}

#[get("/summary")]
pub async fn get_blockchain_summary(
    store: Data<Arc<IndexerStore>>,
    locked_balances: Data<Arc<LockedBalances>>,
) -> HttpResponse {
    let db = store.as_ref();
    if let Ok(Some(best_tip)) = db.get_best_block() {
        trace!("Found best tip: {}", best_tip.summary());

        // accounts
        let total_num_accounts = store
            .get_num_accounts()
            .expect("num accounts")
            .unwrap_or_default();
        let total_num_zkapp_accounts = store
            .get_num_zkapp_accounts()
            .expect("num zkapp accounts")
            .unwrap_or_default();

        // aggregated on-chain & off-chain time-locked tokens
        let chain_id = store.get_chain_id().expect("chain id");
        let genesis_state_hash = store
            .get_block_genesis_state_hash(&best_tip.state_hash())
            .unwrap()
            .expect("genesis state hash");

        let global_slot = best_tip.global_slot_since_genesis();
        let locked_balance = locked_balances.get_locked_amount(global_slot);

        // version info
        let db_version = store.get_db_version().expect("store version");
        let indexer_version = VERSION.to_string();

        // epoch & total data counts
        let epoch_num_blocks = store
            .get_block_production_epoch_count(None, None)
            .expect("epoch blocks count");
        let total_num_blocks = store
            .get_block_production_total_count()
            .expect("total blocks count");

        let epoch_num_canonical_blocks = store
            .get_block_production_canonical_epoch_count(None, None)
            .expect("epoch canonical blocks count");
        let total_num_canonical_blocks = store
            .get_block_production_canonical_total_count()
            .expect("total canonical blocks count");

        let num_unique_block_producers =
            unique_block_producers_last_n_blocks(db, total_num_canonical_blocks)
                .expect("unique block producers");

        let epoch_num_snarks = store
            .get_snarks_epoch_count(None, None)
            .expect("epoch snarks count");
        let total_num_snarks = store.get_snarks_total_count().expect("total snarks count");
        let total_num_canonical_snarks = store
            .get_snarks_total_canonical_count()
            .expect("total canonical snarks count");

        // user commands
        let epoch_num_user_commands = store
            .get_user_commands_epoch_count(None, None)
            .expect("epoch user commands count");
        let total_num_user_commands = store
            .get_user_commands_total_count()
            .expect("total user commands count");

        // applied user commands
        let total_num_applied_user_commands = store
            .get_applied_user_commands_count()
            .expect("total applied user commands count");
        let total_num_canonical_user_commands = store
            .get_canonical_user_commands_count()
            .expect("total canonical user commands count");

        // applied/failed canonical user commands
        let total_num_applied_canonical_user_commands = store
            .get_applied_canonical_user_commands_count()
            .expect("total applied canonical user commands count");
        let total_num_failed_canonical_user_commands = store
            .get_failed_canonical_user_commands_count()
            .expect("total failed canonical user commands count");

        // total failed user commands
        let total_num_failed_user_commands = store
            .get_failed_user_commands_count()
            .expect("total failed user commands count");

        // zkapp commands
        let epoch_num_zkapp_commands = store
            .get_zkapp_commands_epoch_count(None, None)
            .expect("epoch zkapp commands count");
        let total_num_zkapp_commands = store
            .get_zkapp_commands_total_count()
            .expect("total zkapp commands count");

        // applied zkapp commands
        let total_num_applied_zkapp_commands = store
            .get_applied_zkapp_commands_count()
            .expect("total applied zkapp commands count");
        let total_num_canonical_zkapp_commands = store
            .get_canonical_zkapp_commands_count()
            .expect("total canonical zkapp commands count");

        // applied/failed canonical zkapp commands
        let total_num_applied_canonical_zkapp_commands = store
            .get_applied_canonical_zkapp_commands_count()
            .expect("total applied canonical zkapp commands count");
        let total_num_failed_canonical_zkapp_commands = store
            .get_failed_canonical_zkapp_commands_count()
            .expect("total failed canonical zkapp commands count");

        // total failed zkapp commands
        let total_num_failed_zkapp_commands = store
            .get_failed_zkapp_commands_count()
            .expect("total failed zkapp commands count");

        // internal commands
        let epoch_num_internal_commands = store
            .get_internal_commands_epoch_count(None, None)
            .expect("epoch internal commands count");
        let total_num_internal_commands = store
            .get_internal_commands_total_count()
            .expect("total internal commands count");

        // canonical internal commands
        let total_num_canonical_internal_commands = store
            .get_canonical_internal_commands_count()
            .expect("total number of canonical internal commands");

        if let Some(ref summary) = BlockchainSummary::calculate_summary(SummaryInput {
            chain_id,
            genesis_state_hash,

            best_tip,
            locked_balance,
            db_version,
            indexer_version,

            total_num_accounts,
            total_num_zkapp_accounts,

            epoch_num_blocks,
            total_num_blocks,

            epoch_num_canonical_blocks,

            num_unique_block_producers,

            epoch_num_snarks,
            total_num_snarks,
            total_num_canonical_snarks,

            epoch_num_user_commands,
            total_num_user_commands,
            total_num_canonical_user_commands,

            total_num_applied_user_commands,
            total_num_applied_canonical_user_commands,

            total_num_failed_user_commands,
            total_num_failed_canonical_user_commands,

            epoch_num_zkapp_commands,
            total_num_zkapp_commands,
            total_num_canonical_zkapp_commands,

            total_num_applied_zkapp_commands,
            total_num_applied_canonical_zkapp_commands,

            total_num_failed_zkapp_commands,
            total_num_failed_canonical_zkapp_commands,

            epoch_num_internal_commands,
            total_num_internal_commands,
            total_num_canonical_internal_commands,
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
