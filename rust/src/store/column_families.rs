/// Indexer store column family helper trait
use speedb::ColumnFamily;

pub trait ColumnFamilyHelpers {
    ///////////////////////
    // Account store CFs //
    ///////////////////////

    /// CF for storing account balances (best ledger)
    fn account_balance_cf(&self) -> &ColumnFamily;

    /// CF for sorting accounts by balance
    fn account_balance_sort_cf(&self) -> &ColumnFamily;

    /// CF for storing account balance updates
    fn account_balance_updates_cf(&self) -> &ColumnFamily;

    /////////////////////
    // Block store CFs //
    /////////////////////

    /// CF for storing all blocks
    fn blocks_cf(&self) -> &ColumnFamily;

    /// CF for storing block heights
    fn block_height_cf(&self) -> &ColumnFamily;

    /// CF for storing block global slots
    fn block_global_slot_cf(&self) -> &ColumnFamily;

    /// CF for storing block epochs
    fn block_epoch_cf(&self) -> &ColumnFamily;

    /// CF for storing previous state hashes
    fn block_parent_hash_cf(&self) -> &ColumnFamily;

    /// CF for storing block genesis state hashes
    fn block_genesis_state_hash_cf(&self) -> &ColumnFamily;

    /// CF for storing block creators
    fn block_creator_cf(&self) -> &ColumnFamily;

    /// CF for storing coinbase receivers
    fn block_coinbase_receiver_cf(&self) -> &ColumnFamily;

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

    /// CF for sorting block creators by global slot
    fn block_creator_slot_sort_cf(&self) -> &ColumnFamily;

    /// CF for sorting coinbase receivers by block height
    fn block_coinbase_height_sort_cf(&self) -> &ColumnFamily;

    /// CF for sorting coinbase receivers by global slot
    fn block_coinbase_slot_sort_cf(&self) -> &ColumnFamily;

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

    /// CF for storing user commands by `{txn_hash}{state_hash}`
    fn user_commands_cf(&self) -> &ColumnFamily;

    /// CF for storing state hashes of containing blocks
    fn user_command_state_hashes_cf(&self) -> &ColumnFamily;

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

    /// CF for sorting user commands by global slot
    fn user_commands_slot_sort_cf(&self) -> &ColumnFamily;

    /// CF for sorting user commands by sender public key
    fn txn_from_slot_sort_cf(&self) -> &ColumnFamily;

    /// CF for sorting user commands by sender public key
    fn txn_from_height_sort_cf(&self) -> &ColumnFamily;

    /// CF for sorting user commands by receiver public key
    fn txn_to_slot_sort_cf(&self) -> &ColumnFamily;

    /// CF for sorting user commands by sender public key
    fn txn_to_height_sort_cf(&self) -> &ColumnFamily;

    ////////////////////////////////
    // Internal command store CFs //
    ////////////////////////////////

    /// CF for storing internal commands
    fn internal_commands_cf(&self) -> &ColumnFamily;

    /// CF for sorting internal commands by global slot
    fn internal_commands_slot_cf(&self) -> &ColumnFamily;

    //////////////////////
    // Ledger store CFs //
    //////////////////////

    /// CF for storing staged ledgers
    fn ledgers_cf(&self) -> &ColumnFamily;

    /// CF for storing block ledger diffs
    fn block_ledger_diff_cf(&self) -> &ColumnFamily;

    /// CF for storing block staged ledger hashes
    fn block_staged_ledger_hash_cf(&self) -> &ColumnFamily;

    /// CF for storing staking ledgers
    fn staking_ledgers_cf(&self) -> &ColumnFamily;

    /// CF for storing aggregated staking delegations
    fn staking_delegations_cf(&self) -> &ColumnFamily;

    /// CF for storing staking ledger epochs
    fn staking_ledger_hash_to_epoch_cf(&self) -> &ColumnFamily;

    /// CF for storing staking ledger hashes
    fn staking_ledger_epoch_to_hash_cf(&self) -> &ColumnFamily;

    /// CF for storing staking ledger genesis hashes
    fn staking_ledger_genesis_hash_cf(&self) -> &ColumnFamily;

    /// CF for sorting staking ledger accounts by balance
    fn staking_ledger_balance_cf(&self) -> &ColumnFamily;

    /// CF for sorting staking ledger accounts by stake (total delegations)
    fn staking_ledger_stake_cf(&self) -> &ColumnFamily;

    /////////////////////
    // SNARK store CFs //
    /////////////////////

    /// CF for storing SNARKs
    fn snarks_cf(&self) -> &ColumnFamily;

    /// CF for storing all snark work fee totals
    fn snark_top_producers_cf(&self) -> &ColumnFamily;

    /// CF for sorting all snark work fee totals
    fn snark_top_producers_sort_cf(&self) -> &ColumnFamily;

    /// CF for storing/sorting SNARK work fees
    fn snark_work_fees_cf(&self) -> &ColumnFamily;

    /// CF for sorting SNARKs by prover
    fn snark_work_prover_cf(&self) -> &ColumnFamily;

    /// CF for sorting SNARKS by prover and block height
    fn snark_work_prover_height_cf(&self) -> &ColumnFamily;

    ////////////////////
    // Username store //
    ////////////////////

    /// CF for storing update index
    fn username_pk_num_cf(&self) -> &ColumnFamily;

    /// CF for storing indexed usernames
    fn username_pk_index_cf(&self) -> &ColumnFamily;

    /// CF for storing state hash -> usernames
    fn usernames_per_block_cf(&self) -> &ColumnFamily;

    /////////////////
    // Data counts //
    /////////////////

    /// CF for per epoch per account block prodution info
    fn block_production_pk_epoch_cf(&self) -> &ColumnFamily;

    /// CF for per account total block prodution info
    fn block_production_pk_total_cf(&self) -> &ColumnFamily;

    /// CF for per epoch block production totals
    fn block_production_epoch_cf(&self) -> &ColumnFamily;

    /// CF for per block SNARK counts
    fn block_snark_counts_cf(&self) -> &ColumnFamily;

    /// CF for per block user command counts
    fn block_user_command_counts_cf(&self) -> &ColumnFamily;

    /// CF for per block internal command counts
    fn block_internal_command_counts_cf(&self) -> &ColumnFamily;

    /// CF for per epoch user command totals
    fn user_commands_epoch_cf(&self) -> &ColumnFamily;

    /// CF for per epoch per acccount user command totals
    fn user_commands_pk_epoch_cf(&self) -> &ColumnFamily;

    /// CF for per acccount user command totals
    fn user_commands_pk_total_cf(&self) -> &ColumnFamily;

    /// CF for per epoch internal command totals
    fn internal_commands_epoch_cf(&self) -> &ColumnFamily;

    /// CF for per epoch per acccount internal command totals
    fn internal_commands_pk_epoch_cf(&self) -> &ColumnFamily;

    /// CF for per acccount internal command totals
    fn internal_commands_pk_total_cf(&self) -> &ColumnFamily;

    /// CF for per epoch snark totals
    fn snarks_epoch_cf(&self) -> &ColumnFamily;

    /// CF for per epoch snark totals
    fn staking_ledger_accounts_epoch_cf(&self) -> &ColumnFamily;

    /// CF for per epoch per acccount snark totals
    fn snarks_pk_epoch_cf(&self) -> &ColumnFamily;

    /// CF for per acccount snark totals
    fn snarks_pk_total_cf(&self) -> &ColumnFamily;

    /////////////////////
    // Chain store CFs //
    /////////////////////

    /// CF for storing chain_id -> network
    fn chain_id_to_network_cf(&self) -> &ColumnFamily;

    /////////////////////
    // Event store CFs //
    /////////////////////

    /// CF for storing indexer store events by sequence number
    fn events_cf(&self) -> &ColumnFamily;
}
