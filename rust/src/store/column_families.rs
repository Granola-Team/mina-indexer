use speedb::ColumnFamily;

/// Indexer store column family helper trait
pub trait ColumnFamilyHelpers {
    const NUM_COLUMN_FAMILIES: usize = 33;

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

    fn canonicity_cf(&self) -> &ColumnFamily;

    fn commands_cf(&self) -> &ColumnFamily;

    fn internal_commands_cf(&self) -> &ColumnFamily;

    /// CF for sorting user commands by global slot
    fn commands_slot_mainnet_cf(&self) -> &ColumnFamily;

    /// CF for storing global slot by txn hash
    fn commands_txn_hash_to_global_slot_mainnet_cf(&self) -> &ColumnFamily;

    fn ledgers_cf(&self) -> &ColumnFamily;

    fn events_cf(&self) -> &ColumnFamily;

    fn snarks_cf(&self) -> &ColumnFamily;

    /// CF for storing all snark work fee totals
    fn snark_top_producers_cf(&self) -> &ColumnFamily;

    /// CF for sorting all snark work fee totals
    fn snark_top_producers_sort_cf(&self) -> &ColumnFamily;

    /// CF for storing/sorting SNARK work fees
    fn snark_work_fees_cf(&self) -> &ColumnFamily;

    /// CF for storing chain_id -> network
    fn chain_id_to_network_cf(&self) -> &ColumnFamily;

    /// CF for sorting user commands by sender public key
    fn txn_from_cf(&self) -> &ColumnFamily;

    /// CF for sorting user commands by receiver public key
    fn txn_to_cf(&self) -> &ColumnFamily;

    /// CF for per epoch per account block prodution info
    fn block_production_pk_epoch_cf(&self) -> &ColumnFamily;

    /// CF for per account total block prodution info
    fn block_production_pk_total_cf(&self) -> &ColumnFamily;

    /// CF for per epoch block production totals
    fn block_production_epoch_cf(&self) -> &ColumnFamily;

    /// CF for per epoch user command totals
    fn user_commands_epoch_cf(&self) -> &ColumnFamily;

    /// CF for per epoch per acccount user command totals
    fn user_commands_pk_epoch_cf(&self) -> &ColumnFamily;

    /// CF for per acccount user command totals
    fn user_commands_pk_total_cf(&self) -> &ColumnFamily;
}
