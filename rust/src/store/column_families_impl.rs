use crate::store::{column_families::ColumnFamilyHelpers, IndexerStore};
use speedb::ColumnFamily;

impl ColumnFamilyHelpers for IndexerStore {
    /////////////////////
    // Block store CFs //
    /////////////////////

    /// CF for storing blocks
    /// ```
    /// key: state_hash
    /// val: {num block bytes BE u64 bytes}{serde_json block bytes}
    fn blocks_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-state-hash")
            .expect("blocks-state-hash column family exists")
    }

    /// CF for storing PCB versions
    /// ```
    /// key: state hash bytes
    /// val: pcb version serde bytes
    fn block_version_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-version")
            .expect("blocks-version column family exists")
    }

    /// CF for sorting blocks by global slot
    /// ```
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

    /// CF for sorting blocks by block height
    /// ```
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

    fn block_date_time_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-date-time")
            .expect("blocks-date-time column family exists")
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
    /// - txn_hash:   [TxnHash::LEN] bytes
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
    /// - txn_hash:   [TxnHash::LEN] bytes
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
    /// - key: {sender}{block_height}{nonce}{txn_hash}{state_hash}
    /// - val: amount
    /// where
    /// - sender:       [PublicKey::LEN] bytes
    /// - block_height: 4 BE bytes
    /// - nonce:        4 BE bytes
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

    /// Key-value pairs
    /// ```
    /// - key: {state_hash}{index}
    /// - val: [InternalCommandWithData] serde bytes
    /// where
    /// - state_hash: [BlockHash::LEN] bytes
    /// - index:      4 BE bytes
    fn internal_commands_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("internal-commands")
            .expect("internal-commands column family exists")
    }

    /// Key-value pairs
    /// ```
    /// - key: [BlockHash::LEN] bytes
    /// - val: 4 BE bytes
    fn internal_commands_block_num_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("internal-commands-block-num")
            .expect("internal-commands-block-num column family exists")
    }

    /// Key-value pairs
    /// ```
    /// - key: {receiver}{index}
    /// - val: [InternalCommandWithData] serde bytes
    /// where
    /// - receiver:     [PublicKey::LEN] bytes
    /// - index:        4 BE bytes
    fn internal_commands_pk_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("internal-commands-pk")
            .expect("internal-commands-pk column family exists")
    }

    /// Key-value pairs
    /// ```
    /// - key: [PublicKey::LEN] bytes
    /// - val: 4 BE bytes
    fn internal_commands_pk_num_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("internal-commands-pk-num")
            .expect("internal-commands-pk-num column family exists")
    }

    /// Key-value pairs
    /// ```
    /// - key: {block_height}{state_hash}{index}{kind}
    /// - val: [InternalCommandWithData] serde bytes
    /// where
    /// - block_height: 4 BE bytes
    /// - state_hash:   [BlockHash::LEN] bytes
    /// - index:        4 BE bytes
    /// - kind:         0, 1, or 2
    fn internal_commands_block_height_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("internal-commands-block-height-sort")
            .expect("internal-commands-block-height-sort column family exists")
    }

    /// Key-value pairs
    /// ```
    /// - key: {global_slot}{state_hash}{index}
    /// - val: [InternalCommandWithData] serde bytes
    /// where
    /// - global_slot: 4 BE bytes
    /// - state_hash:  [BlockHash::LEN] bytes
    /// - index:       4 BE bytes
    /// - kind:        0, 1, or 2
    fn internal_commands_global_slot_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("internal-commands-global-slot-sort")
            .expect("internal-commands-global-slot-sort column family exists")
    }

    /// Key-value pairs
    /// ```
    /// - key: {receiver}{block_height}{state_hash}{index}
    /// - val: [InternalCommandWithData] serde bytes
    /// where
    /// - receiver:     [PublicKey::LEN] bytes
    /// - block_height: 4 BE bytes
    /// - state_hash:   [BlockHash::LEN] bytes
    /// - index:        4 BE bytes
    fn internal_commands_pk_block_height_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("internal-commands-pk-block-height-sort")
            .expect("internal-commands-pk-block-height-sort column family exists")
    }

    /// Key-value pairs
    /// ```
    /// - key: {receiver}{global_slot}{state_hash}{index}
    /// - val: [InternalCommandWithData] serde bytes
    /// where
    /// - receiver:     [PublicKey::LEN] bytes
    /// - global_slot: 4 BE bytes
    /// - state_hash:   [BlockHash::LEN] bytes
    /// - index:        4 BE bytes
    fn internal_commands_pk_global_slot_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("internal-commands-pk-global-slot-sort")
            .expect("internal-commands-pk-global-slot-sort column family exists")
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

    ///////////////////////////
    // Best ledger store CFs //
    ///////////////////////////

    /// CF for storing best ledger accounts
    /// ```
    /// key: public key bytes
    /// val: account serde bytes
    fn best_ledger_accounts_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("best-ledger-accounts")
            .expect("best-ledger-accounts column family exists")
    }

    /// CF for sorting best ledger accounts by balance
    /// ```
    /// key: {balance}{pk}
    /// val: b""
    /// where
    /// - balance: 8 BE bytes
    /// - pk:      [PublicKey::LEN] bytes
    fn best_ledger_accounts_balance_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("best-ledger-account-balance-sort")
            .expect("best-ledger-account-balance-sort column family exists")
    }

    /// CF for storing number of best ledger account delegations
    /// ```
    /// pk -> num
    /// where
    /// - pk:  [PublicKey::LEN] bytes
    /// - num: 4 BE bytes
    fn best_ledger_accounts_num_delegations_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("best-ledger-account-num-delegations")
            .expect("best-ledger-account-num-delegations column family exists")
    }

    /// CF for storing best ledger account delegations
    /// ```
    /// {pk}{num} -> delegate
    /// where
    /// - pk:  [PublicKey::LEN] bytes
    /// - num: 4 BE bytes
    fn best_ledger_accounts_delegations_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("best-ledger-account-delegations")
            .expect("best-ledger-account-delegations column family exists")
    }

    /////////////////////////////
    // Staged ledger store CFs //
    /////////////////////////////

    /// CF for storing staged ledger accounts (use [staged_account_key])
    /// ```
    /// {state_hash}{pk} -> account
    /// where
    /// - state_hash: [BlockHash::LEN] bytes
    /// - pk:         [PublicKey::LEN] bytes
    /// - account:    serde bytes
    fn staged_ledger_accounts_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staged-ledger-accounts")
            .expect("staged-ledger-accounts column family exists")
    }

    /// CF for sorting staged ledger accounts by balance
    /// ```
    /// {state_hash}{balance}{pk} -> _
    /// where
    /// - state_hash: [BlockHash::LEN] bytes
    /// - balance:    8 BE bytes
    /// - pk:         [PublicKey::LEN] bytes
    fn staged_ledger_account_balance_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staged-ledger-account-balance-sort")
            .expect("staged-ledger-account-balance-sort column family exists")
    }

    /// CF for storing number of staged ledger account delegations
    /// ```
    /// {state_hash}{pk} -> num
    /// where
    /// - state_hash: [BlockHash::LEN] bytes
    /// - pk:         [PublicKey::LEN] bytes
    /// - num:        4 BE bytes
    fn staged_ledger_account_num_delegations_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staged-ledger-account-num-delegations")
            .expect("staged-ledger-account-num-delegations column family exists")
    }

    /// CF for storing staged ledger account delegations
    /// ```
    /// {state_hash}{pk}{num} -> delegate
    /// where
    /// - state_hash: [BlockHash::LEN] bytes
    /// - pk:         [PublicKey::LEN] bytes
    /// - num:        4 BE bytes
    /// - delegate:   serde bytes
    fn staged_ledger_account_delegations_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staged-ledger-account-delegations")
            .expect("staged-ledger-account-delegations column family exists")
    }

    /// CF for storing staged ledger hash -> block state hash
    fn staged_ledger_hash_to_block_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staged-ledger-hash-to-block")
            .expect("staged-ledger-hash-to-block column family exists")
    }

    /// CF for keeping track of which staged ledgers have been persisted
    fn staged_ledgers_persisted_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staged-ledger-persisted")
            .expect("staged-ledger-persisted column family exists")
    }

    /// CF for tracking when an account was added to the staged ledger
    fn staged_ledger_accounts_min_block_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staged-ledger-accounts-min-block")
            .expect("staged-ledger-accounts-min-block column family exists")
    }

    /// CF for storing block ledger diffs
    /// ```
    /// key: state hash bytes
    /// val: ledger diff serde bytes
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

    //////////////////////////////
    // Staking ledger store CFs //
    //////////////////////////////

    /// CF for storing staking ledger accounts
    /// ```
    /// - key: [staking_ledger_account_key]
    /// - val: account serde bytes
    fn staking_ledger_accounts_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staking-ledger-accounts")
            .expect("staking-ledger-accounts column family exists")
    }

    /// CF for storing aggregated staking delegations
    /// ```
    /// - key: [staking_ledger_account_key]
    /// - val: aggregated epoch delegations serde bytes
    fn staking_delegations_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staking-ledger-delegations")
            .expect("staking-ledger-delegations column family exists")
    }

    /// CF for storing aggregated staking delegations
    /// ```
    /// - key: [staking_ledger_epoch_key]
    /// - val: b""
    fn staking_ledger_persisted_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staking-ledger-persisted")
            .expect("staking-ledger-persisted column family exists")
    }

    /// CF for storing staking ledger hashes
    /// ```
    /// - key: [staking_ledger_epoch_key_prefix]
    /// - val: ledger hash bytes
    fn staking_ledger_epoch_to_hash_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staking-ledger-epoch-to-hash")
            .expect("staking-ledger-epoch-to-hash column family exists")
    }

    /// CF for storing staking ledger epochs
    /// ```
    /// - key: ledger hash bytes
    /// - val: epoch BE bytes
    fn staking_ledger_hash_to_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staking-ledger-hash-to-epoch")
            .expect("staking-ledger-hash-to-epoch column family exists")
    }

    /// CF for storing staking ledger genesis state hashes
    /// ```
    /// - key: ledger hash bytes
    /// - val: genesis state hash bytes
    fn staking_ledger_genesis_hash_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staking-ledger-genesis-hash")
            .expect("staking-ledger-genesis-hash column family exists")
    }

    /// CF for storing staking ledger total currencies
    /// ```
    /// - key: ledger hash bytes
    /// - val: total currency BE bytes
    fn staking_ledger_total_currency_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staking-ledger-total-currency")
            .expect("staking-ledger-total-currency column family exists")
    }

    /// CF for sorting staking ledger accounts by balance
    /// ```
    /// - key: [staking_ledger_sort_key]
    /// - val: b""
    fn staking_ledger_balance_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staking-ledger-balance-sort")
            .expect("staking-ledger-balance-sort column family exists")
    }

    /// CF for sorting staking ledger accounts by stake (i.e. total delegations)
    /// ```
    /// - key: [staking_ledger_sort_key]
    /// - val: b""
    fn staking_ledger_stake_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staking-ledger-stake-sort")
            .expect("staking-ledger-stake-sort column family exists")
    }

    /// CF for storing per epoch total number of staking ledger accounts
    /// ```
    /// - key: epoch
    /// - value: number of staking ledger accounts in epoch (4 BE bytes)
    fn staking_ledger_accounts_count_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staking-ledger-accounts-count-epoch")
            .expect("staking-ledger-accounts-count-epoch column family exists")
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
    /// ```
    /// key: {prover}{slot}{index}
    /// val: snark
    /// where
    /// - prover: 55 pk bytes
    /// - slot:   4 BE bytes
    /// - index:  4 BE bytes
    /// - snark:  SNARK serde bytes
    fn snark_work_prover_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snark-work-prover")
            .expect("snark-work-prover column family exists")
    }

    /// CF for storing/sorting SNARKs by prover and block height
    /// ```
    /// key: {prover}{block_height}{index}
    /// val: snark
    /// where
    /// - prover:         55 pk bytes
    /// - block height:   4 BE bytes
    /// - index:          4 BE bytes
    /// - snark:          SNARK serde bytes
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

    /// CF for storing per epoch per account block prodution info
    /// ```
    /// - key: {epoch BE bytes}{pk}
    /// - value: number of blocks produced by pk in epoch
    fn block_production_pk_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-production-pk-epoch")
            .expect("block-production-pk-epoch column family exists")
    }

    /// CF for storing per epoch per account supercharged block prodution info
    /// ```
    /// - key: {epoch BE bytes}{pk}
    /// - value: number of superchargedblocks produced by pk in epoch
    fn block_production_pk_supercharged_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-production-pk-supercharged-epoch")
            .expect("block-production-pk-supercharged-epoch column family exists")
    }

    /// CF for storing per account total block prodution info
    /// ```
    /// - key: pk
    /// - value: total number of blocks produced by pk
    fn block_production_pk_total_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-production-pk-total")
            .expect("block-production-pk-total column family exists")
    }

    /// CF for storing per account total supercharged block prodution info
    /// ```
    /// - key: pk
    /// - value: total number of supercharged blocks produced by pk
    fn block_production_pk_supercharged_total_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-production-pk-supercharged-total")
            .expect("block-production-pk-supercharged-total column family exists")
    }

    /// CF for storing per epoch block production totals
    /// ```
    /// - key: epoch
    /// - value: number of blocks produced in epoch
    fn block_production_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-production-epoch")
            .expect("block-production-epoch column family exists")
    }

    /// CF for storing per epoch block production totals
    /// ```
    /// - key: epoch
    /// - value: number of supercharged blocks produced in epoch
    fn block_production_supercharged_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-production-supercharged-epoch")
            .expect("block-production-supercharged-epoch column family exists")
    }

    /// CF for storing per block SNARK counts
    /// - key: state hash
    /// - value: number of SNARKs in block
    fn block_snark_counts_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-snark-counts")
            .expect("block-snark-counts column family exists")
    }

    /// CF for stoing per block user command counts
    /// ```
    /// - key: state hash
    /// - value: number of user commands in block
    fn block_user_command_counts_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-user-command-counts")
            .expect("block-user-command-counts column family exists")
    }

    /// CF for storing per block internal command counts
    /// ```
    /// - key: state hash
    /// - value: number of internal commands in block
    fn block_internal_command_counts_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-internal-command-counts")
            .expect("block-internal-command-counts column family exists")
    }

    /// CF for storing per epoch per account user commands
    /// ```
    /// - key: {epoch BE bytes}{pk}
    /// - value: number of pk user commands in epoch
    fn user_commands_pk_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("user-commands-pk-epoch")
            .expect("user-commands-pk-epoch column family exists")
    }

    /// CF for storing per account total user commands
    /// ```
    /// - key: pk
    /// - value: total number of pk user commands
    fn user_commands_pk_total_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("user-commands-pk-total")
            .expect("user-commands-pk-total column family exists")
    }

    /// CF for per epoch total user commands
    /// ```
    /// - key: epoch
    /// - value: number of user commands in epoch
    fn user_commands_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("user-commands-epoch")
            .expect("user-commands-epoch column family exists")
    }

    /// CF for storing per epoch per account internal commands
    /// ```
    /// - key: {epoch BE bytes}{pk}
    /// - value: number of pk internal commands in epoch
    fn internal_commands_pk_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("internal-commands-pk-epoch")
            .expect("internal-commands-pk-epoch column family exists")
    }

    /// CF for storing per account total internal commands
    /// ```
    /// - key: pk
    /// - value: total number of pk internal commands
    fn internal_commands_pk_total_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("internal-commands-pk-total")
            .expect("internal-commands-pk-total column family exists")
    }

    /// CF for storing per epoch total internal commands
    /// ```
    /// - key: epoch
    /// - value: number of internal commands in epoch
    fn internal_commands_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("internal-commands-epoch")
            .expect("internal-commands-epoch column family exists")
    }

    /// CF for storing per epoch per account SNARK counts
    /// ```
    /// - key: {epoch BE bytes}{pk}
    /// - value: number of pk SNARKs in epoch
    fn snarks_pk_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snarks-pk-epoch")
            .expect("snarks-pk-epoch column family exists")
    }

    /// CF for storing per account SNARK counts
    /// ```
    /// - key: pk
    /// - value: total number of pk SNARKs
    fn snarks_pk_total_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snarks-pk-total")
            .expect("snarks-pk-total column family exists")
    }

    /// CF for storing per epoch SNARK counts
    /// ```
    /// - key: epoch
    /// - value: number of SNARKs in epoch
    fn snarks_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snarks-epoch")
            .expect("snarks-epoch column family exists")
    }
}
