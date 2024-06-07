/// Indexer store column family helper trait
use speedb::ColumnFamily;

pub trait ColumnFamilyHelpers {
    /// CF for storing account balances (best ledger)
    fn account_balance_cf(&self) -> &ColumnFamily;

    /// CF for sorting accounts by balance
    fn account_balance_sort_cf(&self) -> &ColumnFamily;

    /// CF for storing account balance updates
    fn account_balance_updates_cf(&self) -> &ColumnFamily;

    /// CF for storing all blocks
    fn blocks_cf(&self) -> &ColumnFamily;

    /// CF for storing block versions
    fn blocks_version_cf(&self) -> &ColumnFamily;

    /// CF for sorting blocks by global slot
    fn blocks_global_slot_idx_cf(&self) -> &ColumnFamily;

    /// CF for storing global slot by height
    fn block_height_to_global_slot_cf(&self) -> &ColumnFamily;

    /// CF for storing height by global slot
    fn block_global_slot_to_height_cf(&self) -> &ColumnFamily;

    /// CF for storing previous state hashes
    fn block_parent_hash_cf(&self) -> &ColumnFamily;

    /// CF for storing blockchain lengths
    fn blockchain_length_cf(&self) -> &ColumnFamily;

    /// CF for storing coinbase receivers
    fn coinbase_receiver_cf(&self) -> &ColumnFamily;

    /// CF for storing blocks at a fixed height
    fn lengths_cf(&self) -> &ColumnFamily;

    /// CF for storing blocks at a fixed global slot
    fn slots_cf(&self) -> &ColumnFamily;

    /// CF for storing canonical state hashes by blockchain length
    fn canonicity_length_cf(&self) -> &ColumnFamily;

    /// CF for storing canonical state hashes by global slot
    fn canonicity_slot_cf(&self) -> &ColumnFamily;

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

    /// CF for storing transactions by hash & block order index
    fn user_commands_block_order_cf(&self) -> &ColumnFamily;

    /// CF for sorting user commands by global slot
    fn user_commands_slot_sort_cf(&self) -> &ColumnFamily;

    /// CF for storing global slot by txn hash
    fn user_commands_txn_hash_to_global_slot_cf(&self) -> &ColumnFamily;

    /// CF for storing internal commands
    fn internal_commands_cf(&self) -> &ColumnFamily;

    /// CF for sorting internal commands by global slot
    fn internal_commands_slot_cf(&self) -> &ColumnFamily;

    /// CF for storing staged & staking ledgers
    fn ledgers_cf(&self) -> &ColumnFamily;

    /// CF for storing indexer store events by sequence number
    fn events_cf(&self) -> &ColumnFamily;

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

    /// CF for storing chain_id -> network
    fn chain_id_to_network_cf(&self) -> &ColumnFamily;

    /// CF for sorting user commands by sender public key in [CommandStore]
    fn txn_from_cf(&self) -> &ColumnFamily;

    /// CF for sorting user commands by receiver public key in [CommandStore]
    fn txn_to_cf(&self) -> &ColumnFamily;

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

    /// CF for storing block comparison data
    fn block_comparison_cf(&self) -> &ColumnFamily;

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

    /// CF for per epoch per acccount snark totals
    fn snarks_pk_epoch_cf(&self) -> &ColumnFamily;

    /// CF for per acccount snark totals
    fn snarks_pk_total_cf(&self) -> &ColumnFamily;

    /// CF for storing usernames
    fn usernames_cf(&self) -> &ColumnFamily;

    /// CF for storing state hash -> usernames
    fn usernames_per_block_cf(&self) -> &ColumnFamily;

    /// CF for storing staking ledger epochs
    fn staking_ledger_epoch_cf(&self) -> &ColumnFamily;

    /// CF for sorting staking ledger accounts by balance
    fn staking_ledger_balance_cf(&self) -> &ColumnFamily;

    /// CF for sorting staking ledger accounts by stake (total delegations)
    fn staking_ledger_stake_cf(&self) -> &ColumnFamily;
}
