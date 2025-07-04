//! Column family helpers trait

use speedb::ColumnFamily;

pub trait ColumnFamilyHelpers {
    /////////////////////
    // Block store CFs //
    /////////////////////

    /// CF for storing all blocks
    fn blocks_cf(&self) -> &ColumnFamily;

    /// CF for storing block state hashes
    fn blocks_state_hash_cf(&self) -> &ColumnFamily;

    /// CF for storing block heights
    fn block_height_cf(&self) -> &ColumnFamily;

    /// CF for storing block global slots
    fn block_global_slot_cf(&self) -> &ColumnFamily;

    /// CF for storing block epochs
    fn block_epoch_cf(&self) -> &ColumnFamily;

    /// CF for storing previous state hashes
    fn block_parent_hash_cf(&self) -> &ColumnFamily;

    /// CF for storing block date times
    fn block_date_time_cf(&self) -> &ColumnFamily;

    /// CF for storing block genesis state hashes
    fn block_genesis_state_hash_cf(&self) -> &ColumnFamily;

    /// CF for storing block total supplies
    fn block_total_supply_cf(&self) -> &ColumnFamily;

    /// CF for storing block creators
    fn block_creator_cf(&self) -> &ColumnFamily;

    /// CF for storing coinbase receivers
    fn block_coinbase_receiver_cf(&self) -> &ColumnFamily;

    /// CF for storing block ledger diffs
    fn block_ledger_diff_cf(&self) -> &ColumnFamily;

    /// CF for storing block PCB versions
    fn block_version_cf(&self) -> &ColumnFamily;

    /// CF for storing block comparison data
    fn block_comparison_cf(&self) -> &ColumnFamily;

    /// CF for storing `height -> global slots`
    fn block_height_to_global_slots_cf(&self) -> &ColumnFamily;

    /// CF for storing `global slot -> heights`
    fn block_global_slot_to_heights_cf(&self) -> &ColumnFamily;

    /// CF for sorting block creators by block height
    fn block_creator_height_sort_cf(&self) -> &ColumnFamily;

    /// CF for sorting coinbase receivers by block height
    fn block_coinbase_height_sort_cf(&self) -> &ColumnFamily;

    /// CF for sorting blocks by blockchain length.
    /// Used with [blocks_height_iterator]
    fn blocks_height_sort_cf(&self) -> &ColumnFamily;

    /// CF for sorting blocks by global slot.
    /// Used with [blocks_global_slot_iterator]
    fn blocks_global_slot_sort_cf(&self) -> &ColumnFamily;

    /// CF for storing state hashes of blocks at fixed heights
    fn blocks_at_height_cf(&self) -> &ColumnFamily;

    /// CF for storing state hashes of blocks at fixed global slots
    fn blocks_at_global_slot_cf(&self) -> &ColumnFamily;

    /// CF for storing per epoch slots produced
    fn block_epoch_slots_produced_cf(&self) -> &ColumnFamily;

    /// CF for storing per epoch per account slots produced
    fn block_pk_epoch_slots_produced_cf(&self) -> &ColumnFamily;

    /// CF for storing the number of blocks for a specified public key
    fn blocks_pk_count_cf(&self) -> &ColumnFamily;

    /// CF for storing the tokens used in a blocks
    fn blocks_tokens_used_cf(&self) -> &ColumnFamily;

    //////////////////////////
    // Canonicity store CFs //
    //////////////////////////

    /// CF for storing canonical state hashes by blockchain length
    fn canonicity_length_cf(&self) -> &ColumnFamily;

    /// CF for storing canonical state hashes by global slot
    fn canonicity_slot_cf(&self) -> &ColumnFamily;

    ////////////////////////////
    // User command store CFs //
    ////////////////////////////

    /// CF for storing all user commands
    fn user_commands_cf(&self) -> &ColumnFamily;

    /// CF for storing state hashes of containing blocks
    fn user_commands_state_hashes_cf(&self) -> &ColumnFamily;

    /// CF for storing user commands by state hash
    fn user_commands_per_block_cf(&self) -> &ColumnFamily;

    /// CF for storing user commands by public key
    fn user_commands_pk_cf(&self) -> &ColumnFamily;

    /// CF for storing number of user commands by public key
    fn user_commands_pk_num_cf(&self) -> &ColumnFamily;

    /// CF for storing the number of blocks containing a transaction by hash
    fn user_commands_num_containing_blocks_cf(&self) -> &ColumnFamily;

    /// CF for storing global slot by txn hash
    fn user_commands_txn_hash_to_global_slot_cf(&self) -> &ColumnFamily;

    /// CF for storing blockchain length by txn hash
    fn user_commands_txn_hash_to_block_height_cf(&self) -> &ColumnFamily;

    /// CF for storing transactions by hash & block order index
    fn user_commands_block_order_cf(&self) -> &ColumnFamily;

    /// CF for sorting user commands by blockchain length
    fn user_commands_height_sort_cf(&self) -> &ColumnFamily;

    /// CF for sorting user commands per token by blockchain length
    fn user_commands_per_token_height_sort_cf(&self) -> &ColumnFamily;

    /// CF for sorting user commands by sender public key
    fn txn_from_height_sort_cf(&self) -> &ColumnFamily;

    /// CF for sorting user commands by sender public key
    fn txn_to_height_sort_cf(&self) -> &ColumnFamily;

    // zkapp commands

    /// CF for storing zkapp commands
    fn zkapp_commands_cf(&self) -> &ColumnFamily;

    /// CF for storing zkapp commands by public key
    fn zkapp_commands_pk_cf(&self) -> &ColumnFamily;

    /// CF for storing number of zkapp commands by public key
    fn zkapp_commands_pk_num_cf(&self) -> &ColumnFamily;

    /// CF for sorting zkapp commands by blockchain length
    fn zkapp_commands_height_sort_cf(&self) -> &ColumnFamily;

    /////////////////////
    // Zkapp store CFs //
    /////////////////////

    /// CF for storing zkapp states
    fn zkapp_state_cf(&self) -> &ColumnFamily;

    /// CF for storing zkapp state counts
    fn zkapp_state_num_cf(&self) -> &ColumnFamily;

    /// CF for storing zkapp permissions
    fn zkapp_permissions_cf(&self) -> &ColumnFamily;

    /// CF for storing zkapp permission counts
    fn zkapp_permissions_num_cf(&self) -> &ColumnFamily;

    /// CF for storing zkapp verification keys
    fn zkapp_verification_key_cf(&self) -> &ColumnFamily;

    /// CF for storing zkapp verification key counts
    fn zkapp_verification_key_num_cf(&self) -> &ColumnFamily;

    /// CF for storing zkapp uris
    fn zkapp_uri_cf(&self) -> &ColumnFamily;

    /// CF for storing zkapp uri counts
    fn zkapp_uri_num_cf(&self) -> &ColumnFamily;

    /// CF for storing zkapp token symbols
    fn zkapp_token_symbol_cf(&self) -> &ColumnFamily;

    /// CF for storing zkapp token symbol counts
    fn zkapp_token_symbol_num_cf(&self) -> &ColumnFamily;

    /// CF for storing zkapp timings
    fn zkapp_timing_cf(&self) -> &ColumnFamily;

    /// CF for storing zkapp timing counts
    fn zkapp_timing_num_cf(&self) -> &ColumnFamily;

    /// CF for storing zkapp actions
    fn zkapp_actions_cf(&self) -> &ColumnFamily;

    /// CF for storing a zkapp account's current action count
    fn zkapp_actions_pk_num_cf(&self) -> &ColumnFamily;

    /// CF for storing zkapp events
    fn zkapp_events_cf(&self) -> &ColumnFamily;

    /// CF for storing a zkapp account's current event count
    fn zkapp_events_pk_num_cf(&self) -> &ColumnFamily;

    /// CF for storing tokens
    fn zkapp_tokens_cf(&self) -> &ColumnFamily;

    /// CF for sorting tokens by supply
    fn zkapp_tokens_supply_sort_cf(&self) -> &ColumnFamily;

    /// CF for storing token indexes
    fn zkapp_tokens_index_cf(&self) -> &ColumnFamily;

    /// CF for storing tokens at their indexes
    fn zkapp_tokens_at_index_cf(&self) -> &ColumnFamily;

    /// CF for storing token supplies
    fn zkapp_tokens_supply_cf(&self) -> &ColumnFamily;

    /// CF for storing token owners
    fn zkapp_tokens_owner_cf(&self) -> &ColumnFamily;

    /// CF for storing token symbols
    fn zkapp_tokens_symbol_cf(&self) -> &ColumnFamily;

    /// CF for storing token holders
    fn zkapp_tokens_holder_cf(&self) -> &ColumnFamily;

    /// CF for storing token holder indexes
    fn zkapp_tokens_holder_index_cf(&self) -> &ColumnFamily;

    /// CF for storing token holder counts
    fn zkapp_tokens_holder_count_cf(&self) -> &ColumnFamily;

    /// CF for storing tokens per holder
    fn zkapp_tokens_pk_cf(&self) -> &ColumnFamily;

    /// CF for storing token counts per holder
    fn zkapp_tokens_pk_num_cf(&self) -> &ColumnFamily;

    /// CF for storing token indexes per holder
    fn zkapp_tokens_pk_index_cf(&self) -> &ColumnFamily;

    /// CF for storing token transaction counts per token
    fn zkapp_tokens_txns_num_cf(&self) -> &ColumnFamily;

    /// CF for storing historical token diffs
    fn zkapp_tokens_historical_diffs_cf(&self) -> &ColumnFamily;

    /// CF for storing the count of historical token diffs
    fn zkapp_tokens_historical_diffs_num_cf(&self) -> &ColumnFamily;

    /// CF for storing a public key's historical token diffs
    fn zkapp_tokens_historical_pk_diffs_cf(&self) -> &ColumnFamily;

    /// CF for storing the count of a public key's historical token diffs
    fn zkapp_tokens_historical_pk_diffs_num_cf(&self) -> &ColumnFamily;

    /// CF for storing historical token owners
    fn zkapp_tokens_historical_owners_cf(&self) -> &ColumnFamily;

    /// CF for storing the count of historical token owners
    fn zkapp_tokens_historical_owners_num_cf(&self) -> &ColumnFamily;

    /// CF for storing historical token symbols
    fn zkapp_tokens_historical_symbols_cf(&self) -> &ColumnFamily;

    /// CF for storing the count of historical token symbols
    fn zkapp_tokens_historical_symbols_num_cf(&self) -> &ColumnFamily;

    /// CF for storing historical token supplies
    fn zkapp_tokens_historical_supplies_cf(&self) -> &ColumnFamily;

    /// CF for storing the count of historical token supplies
    fn zkapp_tokens_historical_supplies_num_cf(&self) -> &ColumnFamily;

    ////////////////////////////////
    // Internal command store CFs //
    ////////////////////////////////

    /// CF for storing block internal commands
    fn internal_commands_cf(&self) -> &ColumnFamily;

    /// CF for storing block internal command counts
    fn internal_commands_block_num_cf(&self) -> &ColumnFamily;

    /// CF for storing internal commands by public key
    fn internal_commands_pk_cf(&self) -> &ColumnFamily;

    /// CF for storing internal commands counts by public key
    fn internal_commands_pk_num_cf(&self) -> &ColumnFamily;

    /// CF for sorting internal commands by block height
    fn internal_commands_block_height_sort_cf(&self) -> &ColumnFamily;

    /// CF for sorting internal commands by account & block height
    fn internal_commands_pk_block_height_sort_cf(&self) -> &ColumnFamily;

    ///////////////////////////
    // Best ledger store CFs //
    ///////////////////////////

    /// CF for storing best ledger accounts
    fn best_ledger_accounts_cf(&self) -> &ColumnFamily;

    /// CF for sorting best ledger accounts by balance
    fn best_ledger_accounts_balance_sort_cf(&self) -> &ColumnFamily;

    /// CF for storing zkapp best ledger accounts
    fn zkapp_best_ledger_accounts_cf(&self) -> &ColumnFamily;

    /// CF for sorting zkapp best ledger accounts by balance
    fn zkapp_best_ledger_accounts_balance_sort_cf(&self) -> &ColumnFamily;

    /// CF for storing number of best ledger delegations
    fn best_ledger_accounts_num_delegations_cf(&self) -> &ColumnFamily;

    /// CF for storing best ledger account delegations
    fn best_ledger_accounts_delegations_cf(&self) -> &ColumnFamily;

    /////////////////////////////
    // Staged ledger store CFs //
    /////////////////////////////

    /// CF for storing staged ledger accounts
    fn staged_ledger_accounts_cf(&self) -> &ColumnFamily;

    /// CF for sorting staged ledger accounts by balance
    fn staged_ledger_account_balance_sort_cf(&self) -> &ColumnFamily;

    /// CF for storing number of staged ledger delegations
    fn staged_ledger_account_num_delegations_cf(&self) -> &ColumnFamily;

    /// CF for storing staged ledger account delegations
    fn staged_ledger_account_delegations_cf(&self) -> &ColumnFamily;

    /// CF for storing staged ledger hash -> state hash
    fn staged_ledger_hash_to_block_cf(&self) -> &ColumnFamily;

    /// CF for storing which staged ledgers have been persisted
    fn staged_ledgers_persisted_cf(&self) -> &ColumnFamily;

    /// CF for tracking when an account was added to the staged ledger
    fn staged_ledger_accounts_min_block_cf(&self) -> &ColumnFamily;

    /// CF for storing block staged ledger hashes
    /// state hash -> staged ledger hash
    fn block_staged_ledger_hash_cf(&self) -> &ColumnFamily;

    //////////////////////////////
    // Staking ledger store CFs //
    //////////////////////////////

    /// CF for storing staking ledgers
    fn staking_ledger_accounts_cf(&self) -> &ColumnFamily;

    /// CF for storing aggregated staking delegations
    fn staking_delegations_cf(&self) -> &ColumnFamily;

    /// CF for tracking persisted staking ledgers
    fn staking_ledger_persisted_cf(&self) -> &ColumnFamily;

    /// CF for storing staking ledger epochs
    fn staking_ledger_hash_to_epoch_cf(&self) -> &ColumnFamily;

    /// CF for storing staking ledger hashes
    fn staking_ledger_epoch_to_hash_cf(&self) -> &ColumnFamily;

    /// CF for storing staking ledger genesis hashes
    fn staking_ledger_genesis_hash_cf(&self) -> &ColumnFamily;

    /// CF for storing staking ledger total currencies
    fn staking_ledger_total_currency_cf(&self) -> &ColumnFamily;

    /// CF for sorting staking ledger accounts by balance
    fn staking_ledger_balance_sort_cf(&self) -> &ColumnFamily;

    /// CF for sorting staking ledger accounts by stake (total delegations)
    fn staking_ledger_stake_sort_cf(&self) -> &ColumnFamily;

    /// CF for per epoch staking account totals
    fn staking_ledger_accounts_count_epoch_cf(&self) -> &ColumnFamily;

    /////////////////////
    // SNARK store CFs //
    /////////////////////

    /// CF for storing SNARKs by block state hash & index
    /// [block_snark_counts_cf] stores the number of SNARKs per block
    fn snarks_cf(&self) -> &ColumnFamily;

    /// CF for storing SNARKs per prover & index
    /// [snarks_pk_total_cf] stores per prover SNARK counts
    fn snarks_prover_cf(&self) -> &ColumnFamily;

    /// CF for storing all SNARK prover fee totals
    fn snark_prover_fees_cf(&self) -> &ColumnFamily;

    /// CF for storing per epoch SNARK prover fee totals
    fn snark_prover_fees_epoch_cf(&self) -> &ColumnFamily;

    /// CF for storing per block height all-time SNARK prover fee updates
    fn snark_prover_fees_historical_cf(&self) -> &ColumnFamily;

    /// CF for storing per block height epoch SNARK prover fee updates
    fn snark_prover_fees_epoch_historical_cf(&self) -> &ColumnFamily;

    /// CF for sorting all SNARK prover fee totals
    fn snark_prover_total_fees_sort_cf(&self) -> &ColumnFamily;

    /// CF for sorting per epoch SNARK prover fee totals
    fn snark_prover_total_fees_epoch_sort_cf(&self) -> &ColumnFamily;

    /// CF for storing SNARK prover max fees
    fn snark_prover_max_fee_cf(&self) -> &ColumnFamily;

    /// CF for storing per epoch SNARK prover max fees
    fn snark_prover_max_fee_epoch_cf(&self) -> &ColumnFamily;

    /// CF for sorting all SNARK provers by max fees
    fn snark_prover_max_fee_sort_cf(&self) -> &ColumnFamily;

    /// CF for sorting per epoch SNARK provers by max fees
    fn snark_prover_max_fee_epoch_sort_cf(&self) -> &ColumnFamily;

    /// CF for storing SNARK prover min fees
    fn snark_prover_min_fee_cf(&self) -> &ColumnFamily;

    /// CF for storing per epoch SNARK prover min fees
    fn snark_prover_min_fee_epoch_cf(&self) -> &ColumnFamily;

    /// CF for sorting all SNARK provers by min fees
    fn snark_prover_min_fee_sort_cf(&self) -> &ColumnFamily;

    /// CF for sorting per epoch SNARK provers by min fees
    fn snark_prover_min_fee_epoch_sort_cf(&self) -> &ColumnFamily;

    /// CF for sorting SNARKS by prover & block height
    fn snark_prover_block_height_sort_cf(&self) -> &ColumnFamily;

    /// CF for sorting SNARK work fees & block height
    fn snark_work_fees_block_height_sort_cf(&self) -> &ColumnFamily;

    ////////////////////
    // Username store //
    ////////////////////

    /// CF for storing username update count
    fn username_num_cf(&self) -> &ColumnFamily;

    /// CF for storing usernames per pk
    fn username_cf(&self) -> &ColumnFamily;

    /// CF for storing usernames per block
    fn usernames_per_block_cf(&self) -> &ColumnFamily;

    /// CF for storing pk's per username
    fn username_pk_cf(&self) -> &ColumnFamily;

    /////////////////
    // Data counts //
    /////////////////

    /// CF for per epoch per account block prodution info
    fn block_production_pk_epoch_cf(&self) -> &ColumnFamily;

    /// CF for per epoch per account canonical block prodution info
    fn block_production_pk_canonical_epoch_cf(&self) -> &ColumnFamily;

    /// CF for sorting per epoch per account canonical block prodution info
    fn block_production_pk_canonical_epoch_sort_cf(&self) -> &ColumnFamily;

    /// CF for per epoch per account supercharged block prodution info
    fn block_production_pk_supercharged_epoch_cf(&self) -> &ColumnFamily;

    /// CF for per account total block prodution info
    fn block_production_pk_total_cf(&self) -> &ColumnFamily;

    /// CF for per account total canonical block prodution info
    fn block_production_pk_canonical_total_cf(&self) -> &ColumnFamily;

    /// CF for per account total supercharged block prodution info
    fn block_production_pk_supercharged_total_cf(&self) -> &ColumnFamily;

    /// CF for per epoch block production totals
    fn block_production_epoch_cf(&self) -> &ColumnFamily;

    /// CF for per epoch canonical block production totals
    fn block_production_canonical_epoch_cf(&self) -> &ColumnFamily;

    /// CF for per epoch supercharged block production totals
    fn block_production_supercharged_epoch_cf(&self) -> &ColumnFamily;

    /// CF for per block SNARK counts
    fn block_snark_counts_cf(&self) -> &ColumnFamily;

    /// CF for per block user command counts
    fn block_user_command_counts_cf(&self) -> &ColumnFamily;

    /// CF for per block zkapp command counts
    fn block_zkapp_command_counts_cf(&self) -> &ColumnFamily;

    /// CF for per block internal command counts
    fn block_internal_command_counts_cf(&self) -> &ColumnFamily;

    /// CF for per epoch slots produced counts
    fn block_epoch_slots_produced_count_cf(&self) -> &ColumnFamily;

    /// CF for per epoch per account slots produced counts
    fn block_pk_epoch_slots_produced_count_cf(&self) -> &ColumnFamily;

    /// CF for sorting by per epoch per account slots produced counts
    fn block_pk_epoch_slots_produced_count_sort_cf(&self) -> &ColumnFamily;

    /// CF for per epoch user command totals
    fn user_commands_epoch_cf(&self) -> &ColumnFamily;

    /// CF for per epoch zkapp command totals
    fn zkapp_commands_epoch_cf(&self) -> &ColumnFamily;

    /// CF for per epoch per acccount user command totals
    fn user_commands_pk_epoch_cf(&self) -> &ColumnFamily;

    /// CF for per epoch per acccount zkapp command totals
    fn zkapp_commands_pk_epoch_cf(&self) -> &ColumnFamily;

    /// CF for per acccount user command totals
    fn user_commands_pk_total_cf(&self) -> &ColumnFamily;

    /// CF for per acccount zkapp command totals
    fn zkapp_commands_pk_total_cf(&self) -> &ColumnFamily;

    /// CF for per epoch internal command totals
    fn internal_commands_epoch_cf(&self) -> &ColumnFamily;

    /// CF for per epoch per acccount internal command totals
    fn internal_commands_pk_epoch_cf(&self) -> &ColumnFamily;

    /// CF for per acccount internal command totals
    fn internal_commands_pk_total_cf(&self) -> &ColumnFamily;

    /// CF for per epoch SNARK totals
    fn snarks_epoch_cf(&self) -> &ColumnFamily;

    /// CF for per epoch per acccount SNARK totals
    fn snarks_pk_epoch_cf(&self) -> &ColumnFamily;

    /// CF for per acccount SNARK totals
    fn snarks_pk_total_cf(&self) -> &ColumnFamily;

    /////////////////////
    // Chain store CFs //
    /////////////////////

    /// CF for storing chain_id -> network
    fn chain_id_to_network_cf(&self) -> &ColumnFamily;

    /////////////////////////////
    // Indexer event store CFs //
    /////////////////////////////

    /// CF for storing indexer store events by sequence number
    fn events_cf(&self) -> &ColumnFamily;
}
