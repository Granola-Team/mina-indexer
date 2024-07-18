use crate::store::{column_families::ColumnFamilyHelpers, IndexerStore};
use speedb::ColumnFamily;

impl ColumnFamilyHelpers for IndexerStore {
    ///////////////////////
    // Account store CFs //
    ///////////////////////

    /// `pk -> balance`
    fn account_balance_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("account-balance")
            .expect("account-balance column family exists")
    }

    /// CF for sorting account's by balance
    /// `{balance}{pk} -> _`
    ///
    /// - `balance`: 8 BE bytes
    fn account_balance_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("account-balance-sort")
            .expect("account-balance-sort column family exists")
    }

    /// 'state_hash -> balance updates`
    fn account_balance_updates_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("account-balance-updates")
            .expect("account-balance-updates column family exists")
    }

    /////////////////////
    // Block store CFs //
    /////////////////////

    /// Blocks CF
    /// ```
    /// state_hash -> {num block bytes BE u64 bytes}{serde_json block bytes}
    fn blocks_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-state-hash")
            .expect("blocks-state-hash column family exists")
    }

    /// `state_hash -> pcb version`
    fn block_version_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-version")
            .expect("blocks-version column family exists")
    }

    /// ```
    /// --------------------------------
    /// - key: {global_slot}{state_hash}
    /// - val: b""
    /// where
    /// - global_slot: 4 BE bytes
    /// - state_hash:  [BlockHash::LEN] bytes
    fn blocks_global_slot_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-global-slot-sort")
            .expect("blocks-global-slot-sort column family exists")
    }

    /// ```
    /// ---------------------------------
    /// - key: {block_height}{state_hash}
    /// - val: b""
    /// where
    /// - block_height: 4 BE bytes
    /// - state_hash:   [BlockHash::LEN] bytes
    fn blocks_height_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-height-sort")
            .expect("blocks-height-sort column family exists")
    }

    fn block_height_to_global_slots_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-height-to-slots")
            .expect("blocks-height-to-slots column family exists")
    }

    fn block_global_slot_to_heights_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-slot-to-heights")
            .expect("blocks-slot-to-heights column family exists")
    }

    fn block_parent_hash_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-parent-hash")
            .expect("blocks-parent-hash column family exists")
    }

    fn block_height_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-height")
            .expect("blocks-height column family exists")
    }

    fn block_global_slot_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-global-slot")
            .expect("blocks-global-slot column family exists")
    }

    fn block_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-epoch")
            .expect("blocks-epoch column family exists")
    }

    fn block_genesis_state_hash_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-genesis-hash")
            .expect("blocks-genesis-hash column family exists")
    }

    fn block_creator_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-creator")
            .expect("blocks-creator column family exists")
    }

    fn block_coinbase_receiver_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-coinbase-receiver")
            .expect("blocks-coinbase-receiver column family exists")
    }

    fn block_coinbase_height_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("coinbase-receiver-height-sort")
            .expect("coinbase-receiver-height-sort column family exists")
    }

    fn block_coinbase_slot_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("coinbase-receiver-slot-sort")
            .expect("coinbase-receiver-slot-sort column family exists")
    }

    fn block_creator_height_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-creator-height-sort")
            .expect("block-creator-height-sort column family exists")
    }

    fn block_creator_slot_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-creator-slot-sort")
            .expect("block-creator-slot-sort column family exists")
    }

    /// CF for storing blocks at a fixed height:
    /// `height -> list of state hashes at height`
    ///
    /// - `list of state hashes at height`: sorted from best to worst
    fn blocks_at_height_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-at-length")
            .expect("blocks-at-length column family exists")
    }

    /// CF for storing blocks at a fixed global slot:
    /// `global slot -> list of state hashes at slot`
    ///
    /// - `list of state hashes at slot`: sorted from best to worst
    fn blocks_at_global_slot_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-at-slot")
            .expect("blocks-at-slot column family exists")
    }

    fn block_comparison_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-comparison")
            .expect("blocks-comparison column family exists")
    }

    ////////////////////////////
    // User command store CFs //
    ////////////////////////////

    fn user_commands_pk_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("user-commands-pk")
            .expect("user-commands-pk column family exists")
    }

    fn user_commands_pk_num_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("user-commands-pk-num")
            .expect("user-commands-pk-num column family exists")
    }

    fn user_command_state_hashes_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("user-command-state-hashes")
            .expect("user-command-state-hashes column family exists")
    }

    fn user_commands_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("user-commands")
            .expect("user-commands column family exists")
    }

    fn user_commands_per_block_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("user-commands-block")
            .expect("user-commands-block column family exists")
    }

    fn user_commands_block_order_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("user-commands-block-order")
            .expect("user-commands-block-order column family exists")
    }

    fn user_commands_num_containing_blocks_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("user-commands-num-blocks")
            .expect("user-commands-num-blocks column family exists")
    }

    /// Key-value pairs
    /// ```
    /// - key: {height}{txn_hash}{state_hash}
    /// - val: b""
    /// where
    /// - height:     4 BE bytes
    /// - txn_hash:   [TXN_HASH_LEN] bytes
    /// - state_hash: [BlockHash::LEN] bytes
    fn user_commands_height_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("user-commands-height-sort")
            .expect("user-commands-height-sort column family exists")
    }

    /// Key-value pairs
    /// ```
    /// - key: {slot}{txn_hash}{state_hash}
    /// - val: b""
    /// where
    /// - slot:       4 BE bytes
    /// - txn_hash:   [TXN_HASH_LEN] bytes
    /// - state_hash: [BlockHash::LEN] bytes
    fn user_commands_slot_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("user-commands-slot-sort")
            .expect("user-commands-slot-sort column family exists")
    }

    /// Key-value pairs
    /// ```
    /// - key: txn_hash
    /// - val: blockchain_length
    /// where
    /// - blockchain_length: 4 BE bytes
    fn user_commands_txn_hash_to_block_height_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("user-commands-to-block-height")
            .expect("user-commands-to-block-height column family exists")
    }

    /// Key-value pairs
    /// ```
    /// - key: txn_hash
    /// - val: global_slot
    /// where
    /// - global_slot: 4 BE bytes
    fn user_commands_txn_hash_to_global_slot_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("user-commands-to-global-slot")
            .expect("user-commands-to-global-slot column family exists")
    }

    /// Key-value pairs
    /// ```
    /// - key: {sender}{global_slot}{txn_hash}{state_hash}
    /// - val: amount
    /// where
    /// - sender:      [PublicKey::LEN] bytes
    /// - global_slot: 4 BE bytes
    /// - txn_hash:    [TX_HASH_LEN] bytes
    /// - state_hash:  [BlockHash::LEN] bytes
    /// - amount:      8 BE bytes
    fn txn_from_slot_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("txn-from-slot-sort")
            .expect("txn-from-slot-sort column family exists")
    }

    /// Key-value pairs
    /// ```
    /// - key: {sender}{block_height}{txn_hash}{state_hash}
    /// - val: amount
    /// where
    /// - sender:       [PublicKey::LEN] bytes
    /// - block_height: 4 BE bytes
    /// - txn_hash:     [TX_HASH_LEN] bytes
    /// - state_hash:   [BlockHash::LEN] bytes
    /// - amount:       8 BE bytes
    fn txn_from_height_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("txn-from-height-sort")
            .expect("txn-from-height-sort column family exists")
    }

    /// Key-value pairs
    /// ```
    /// - key: {receiver}{global_slot}{txn_hash}{state_hash}
    /// - val: amount
    /// where
    /// - receiver:    [PublicKey::LEN] bytes
    /// - global_slot: 4 BE bytes
    /// - txn_hash:    [TX_HASH_LEN] bytes
    /// - state_hash:  [BlockHash::LEN] bytes
    /// - amount:      8 BE bytes
    fn txn_to_slot_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("txn-to-slot-sort")
            .expect("txn-to-slot-sort column family exists")
    }

    /// Key-value pairs
    /// ```
    /// - key: {receiver}{block_height}{txn_hash}{state_hash}
    /// - val: amount
    /// where
    /// - receiver:     [PublicKey::LEN] bytes
    /// - block_height: 4 BE bytes
    /// - txn_hash:     [TX_HASH_LEN] bytes
    /// - state_hash:   [BlockHash::LEN] bytes
    /// - amount:       8 BE bytes
    fn txn_to_height_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("txn-to-height-sort")
            .expect("txn-to-height-sort column family exists")
    }

    ////////////////////////////////
    // Internal command store CFs //
    ////////////////////////////////

    fn internal_commands_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("internal-commands")
            .expect("internal-commands column family exists")
    }

    fn internal_commands_slot_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("internal-commands-global-slot")
            .expect("internal-commands-global-slot column family exists")
    }

    //////////////////////////
    // Canonicity store CFs //
    //////////////////////////

    fn canonicity_length_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("canonicity-length")
            .expect("canonicity-length column family exists")
    }

    fn canonicity_slot_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("canonicity-slot")
            .expect("canonicity-slot column family exists")
    }

    //////////////////////
    // Ledger store CFs //
    //////////////////////

    fn ledgers_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("ledgers")
            .expect("ledgers column family exists")
    }

    fn block_ledger_diff_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-ledger-diff")
            .expect("blocks-ledger-diff column family exists")
    }

    fn block_staged_ledger_hash_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-staged-ledger-hash")
            .expect("blocks-staged-ledger-hash column family exists")
    }

    /// CF for storing staking ledgers
    /// ```
    /// - key: {genesis_hash}{epoch}{ledger_hash}
    /// - val: staking ledger
    /// where
    /// - genesis_hash: [BlockHash::LEN] bytes
    /// - epoch:        4 BE bytes
    /// - ledger_hash:  [TXN_HASH_LEN] bytes
    fn staking_ledgers_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staking-ledgers")
            .expect("staking-ledgers column family exists")
    }

    /// CF for storing staking ledger hashes
    /// ```
    /// - key: epoch
    /// - val: ledger hash
    /// where
    /// - epoch:        4 BE bytes
    /// - ledger hash:  [TXN_HASH_LEN] bytes
    fn staking_ledger_epoch_to_hash_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staking-ledger-epoch-to-hash")
            .expect("staking-ledger-epoch-to-hash column family exists")
    }

    /// CF for storing staking ledger epochs
    /// ```
    /// - key: ledger hash
    /// - val: epoch
    /// where
    /// - ledger hash: [TXN_HASH_LEN] bytes
    /// - epoch:       4 BE bytes
    fn staking_ledger_hash_to_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staking-ledger-hash-to-epoch")
            .expect("staking-ledger-hash-to-epoch column family exists")
    }

    /// CF for storing staking ledger genesis state hashes
    /// ```
    /// - key: ledger_hash
    /// - val: genesis_hash
    /// where
    /// - ledger_hash:  [TXN_HASH_LEN] bytes
    /// - genesis_hash: [BlockHash::LEN] bytes
    fn staking_ledger_genesis_hash_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staking-ledger-genesis-hash")
            .expect("staking-ledger-genesis-hash column family exists")
    }

    /// CF for storing aggregated staking delegations
    /// ```
    /// - key: {genesis_hash}{epoch}
    /// - val: aggregated epoch delegations
    /// where
    /// - genesis_hash: [BlockHash::LEN] bytes
    /// - epoch:        4 BE bytes
    fn staking_delegations_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staking-delegations")
            .expect("staking-delegations column family exists")
    }

    /// Key-value pairs
    /// ```
    /// - key: {balance}{pk}
    /// - val: b""
    /// where
    /// - balance: 8 BE bytes
    /// - pk:      [PublicKey::LEN] bytes
    fn staking_ledger_balance_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staking-ledger-balance")
            .expect("staking-ledger-balance column family exists")
    }

    /// Key-value pairs
    /// ```
    /// - key: {stake}{pk}
    /// - val: b""
    /// where
    /// - stake: 8 BE bytes
    /// - pk:    [PublicKey::LEN] bytes
    fn staking_ledger_stake_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staking-ledger-stake")
            .expect("staking-ledger-stake column family exists")
    }

    /////////////////////
    // SNARK store CFs //
    /////////////////////

    fn snarks_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snarks")
            .expect("snarks column family exists")
    }

    fn snark_top_producers_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snark-work-top-producers")
            .expect("snark-work-top-producers column family exists")
    }

    fn snark_top_producers_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snark-work-top-producers-sort")
            .expect("snark-work-top-producers-sort column family exists")
    }

    /// key: [snark_fee_prefix_key]
    fn snark_work_fees_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snark-work-fees")
            .expect("snark-work-fees column family exists")
    }

    /// CF for storing/sorting SNARKs by prover
    /// `{prover}{slot}{index} -> snark`
    /// - prover: 55 pk bytes
    /// - slot:   4 BE bytes
    /// - index:  4 BE bytes
    fn snark_work_prover_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snark-work-prover")
            .expect("snark-work-prover column family exists")
    }

    /// CF for storing/sorting SNARKs by prover and block height
    /// `{prover}{block_height}{index} -> snark`
    /// - prover:         55 pk bytes
    /// - block height:   4 BE bytes
    /// - index:          4 BE bytes
    fn snark_work_prover_height_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snark-work-prover-height")
            .expect("snark-work-prover column family exists")
    }

    ////////////////////////
    // Username store CFs //
    ////////////////////////

    fn username_pk_num_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("username-pk-num")
            .expect("username-pk-num column family exists")
    }

    fn username_pk_index_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("username-pk-index")
            .expect("username-pk-index column family exists")
    }

    /// CF for storing state hash -> usernames
    fn usernames_per_block_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("usernames-per-block")
            .expect("usernames-per-block column family exists")
    }

    /////////////////////
    // Chain store CFs //
    /////////////////////

    fn chain_id_to_network_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("chain-id-to-network")
            .expect("chain-id-to-network column family exists")
    }

    /////////////////////
    // Event store CFs //
    /////////////////////

    fn events_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("events")
            .expect("events column family exists")
    }

    ////////////////////
    // Data count CFs //
    ////////////////////

    /// CF for per epoch per account block prodution info
    /// - key: `{epoch BE bytes}{pk}`
    /// - value: number of blocks produced by `pk` in `epoch`
    fn block_production_pk_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-production-pk-epoch")
            .expect("block-production-pk-epoch column family exists")
    }

    /// CF for per account total block prodution info
    /// - key: pk
    /// - value: total number of blocks produced by pk
    fn block_production_pk_total_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-production-pk-total")
            .expect("block-production-pk-total column family exists")
    }

    /// CF for per epoch block production counts
    /// - key: epoch
    /// - value: number of blocks produced in epoch
    fn block_production_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-production-epoch")
            .expect("block-production-epoch column family exists")
    }

    /// CF for per block SNARK counts
    /// - key: state hash
    /// - value: number of SNARKs in block
    fn block_snark_counts_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-snark-counts")
            .expect("block-snark-counts column family exists")
    }

    /// CF for per block user command counts
    /// - key: state hash
    /// - value: number of user commands in block
    fn block_user_command_counts_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-user-command-counts")
            .expect("block-user-command-counts column family exists")
    }

    /// CF for per block internal command counts
    /// - key: state hash
    /// - value: number of internal commands in block
    fn block_internal_command_counts_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-internal-command-counts")
            .expect("block-internal-command-counts column family exists")
    }

    /// CF for per epoch per account user commands
    /// - key: `{epoch BE bytes}{pk}`
    /// - value: number of `pk` user commands in `epoch`
    fn user_commands_pk_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("user-commands-pk-epoch")
            .expect("user-commands-pk-epoch column family exists")
    }

    /// CF for per account total user commands
    /// - key: `pk`
    /// - value: total number of `pk` user commands
    fn user_commands_pk_total_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("user-commands-pk-total")
            .expect("user-commands-pk-total column family exists")
    }

    /// CF for per epoch total user commands
    /// - key: `epoch`
    /// - value: number of user commands in `epoch`
    fn user_commands_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("user-commands-epoch")
            .expect("user-commands-epoch column family exists")
    }

    /// CF for per epoch per account internal commands
    /// - key: `{epoch BE bytes}{pk}`
    /// - value: number of `pk` internal commands in `epoch`
    fn internal_commands_pk_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("internal-commands-pk-epoch")
            .expect("internal-commands-pk-epoch column family exists")
    }

    /// CF for per account total internal commands
    /// - key: `pk`
    /// - value: total number of `pk` internal commands
    fn internal_commands_pk_total_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("internal-commands-pk-total")
            .expect("internal-commands-pk-total column family exists")
    }

    /// CF for per epoch total internal commands
    /// - key: `epoch`
    /// - value: number of internal commands in `epoch`
    fn internal_commands_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("internal-commands-epoch")
            .expect("internal-commands-epoch column family exists")
    }

    /// CF for per epoch per account SNARKs
    /// - key: `{epoch BE bytes}{pk}`
    /// - value: number of `pk` SNARKs in `epoch`
    fn snarks_pk_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snarks-pk-epoch")
            .expect("snarks-pk-epoch column family exists")
    }

    /// CF for per account total SNARKs
    /// - key: `pk`
    /// - value: total number of `pk` SNARKs
    fn snarks_pk_total_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snarks-pk-total")
            .expect("snarks-pk-total column family exists")
    }

    /// CF for per epoch total SNARKs
    /// - key: `epoch`
    /// - value: number of SNARKs in `epoch`
    fn snarks_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snarks-epoch")
            .expect("snarks-epoch column family exists")
    }

    /// CF for per epoch total staking ledger accounts
    /// - key: `epoch`
    /// - value: number of staking ledgers in `epoch`
    fn staking_ledger_accounts_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staking-ledger-accounts-epoch")
            .expect("staking-ledger-accounts-epoch column family exists")
    }
}
