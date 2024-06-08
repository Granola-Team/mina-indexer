use crate::store::{column_families::ColumnFamilyHelpers, IndexerStore};
use speedb::ColumnFamily;

impl ColumnFamilyHelpers for IndexerStore {
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

    /// `state_hash -> block`
    fn blocks_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-state-hash")
            .expect("blocks-state-hash column family exists")
    }

    /// `state_hash -> pcb version`
    fn blocks_version_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-version")
            .expect("blocks-version column family exists")
    }

    /// `{global_slot}{state_hash} -> _`
    fn blocks_global_slot_idx_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-global-slot-idx")
            .expect("blocks-global-slot-idx column family exists")
    }

    fn block_height_to_global_slot_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-height-to-slot")
            .expect("block-height-to-slot column family exists")
    }

    fn block_global_slot_to_height_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-slot-to-height")
            .expect("block-slot-to-height column family exists")
    }

    fn block_parent_hash_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-parent-hash")
            .expect("block-parent-hash column family exists")
    }

    fn blockchain_length_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blockchain-length")
            .expect("blockchain-length column family exists")
    }

    fn coinbase_receiver_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("coinbase-receivers")
            .expect("coinbase-receivers column family exists")
    }

    /// CF for storing blocks at a fixed height:
    /// `height -> list of state hashes at height`
    ///
    /// - `list of state hashes at height`: sorted from best to worst
    fn lengths_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-at-length")
            .expect("blocks-at-length column family exists")
    }

    /// CF for storing blocks at a fixed global slot:
    /// `global slot -> list of state hashes at slot`
    ///
    /// - `list of state hashes at slot`: sorted from best to worst
    fn slots_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-at-slot")
            .expect("blocks-at-slot column family exists")
    }

    fn block_comparison_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-comparison")
            .expect("block-comparison column family exists")
    }

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

    /// **Key format:** `{slot}{txn_hash}{state_hash}`
    /// ```
    /// - slot:       4 BE bytes
    /// - txn_hash:   [TXN_HASH_LEN] bytes
    /// - state_hash: [BlockHash::LEN] bytes
    fn user_commands_slot_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("user-commands-slot-sort")
            .expect("user-commands-slot-sort column family exists")
    }

    /// `txn_hash -> global_slot`
    ///
    /// - `global_slot`: 4 BE bytes
    fn user_commands_txn_hash_to_global_slot_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("user-commands-to-global-slot")
            .expect("user-commands-to-global-slot column family exists")
    }

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

    fn ledgers_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("ledgers")
            .expect("ledgers column family exists")
    }

    fn events_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("events")
            .expect("events column family exists")
    }

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

    fn chain_id_to_network_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("chain-id-to-network")
            .expect("chain-id-to-network column family exists")
    }

    fn txn_from_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("txn-from")
            .expect("txn-from column family exists")
    }

    fn txn_to_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("txn-to")
            .expect("txn-to column family exists")
    }

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

    /// CF for storing usernames
    fn usernames_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("usernames")
            .expect("usernames column family exists")
    }

    /// CF for storing state hash -> usernames
    fn usernames_per_block_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("usernames-per-block")
            .expect("usernames-per-block column family exists")
    }

    /// CF for storing staking ledger epochs
    fn staking_ledger_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staking-ledger-epoch")
            .expect("staking-ledger-epoch column family exists")
    }

    /// CF for sorting staking ledger accounts by balance
    fn staking_ledger_balance_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staking-ledger-balance")
            .expect("staking-ledger-balance column family exists")
    }

    /// CF for sorting staking ledger accounts by total delegations
    fn staking_ledger_stake_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staking-ledger-stake")
            .expect("staking-ledger-stake column family exists")
    }
}
