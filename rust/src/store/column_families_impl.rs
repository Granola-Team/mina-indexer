use crate::store::{column_families::ColumnFamilyHelpers, IndexerStore};
use speedb::ColumnFamily;

impl ColumnFamilyHelpers for IndexerStore {
    /// CF for storing account balances (best ledger):
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

    /// CF for storing account balance updates:
    /// `state hash -> balance updates`
    fn account_balance_updates_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("account-balance-updates")
            .expect("account-balance-updates column family exists")
    }

    /// CF for storing all blocks
    fn blocks_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-state-hash")
            .expect("blocks-state-hash column family exists")
    }

    /// CF for storing block versions:
    /// `state hash -> pcb version`
    fn blocks_version_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-version")
            .expect("blocks-version column family exists")
    }

    /// CF for sorting blocks by global slot
    /// `{global_slot}{state_hash} -> _`
    fn blocks_global_slot_idx_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-global-slot-idx")
            .expect("blocks-global-slot-idx column family exists")
    }

    /// CF for storing: height -> global slot
    fn block_height_to_global_slot_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-height-to-slot")
            .expect("block-height-to-slot column family exists")
    }

    /// CF for storing: global slot -> height
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
    /// `height -> list of blocks at height`
    ///
    /// - `list of blocks at height`: sorted from best to worst
    fn lengths_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-at-length")
            .expect("blocks-at-length column family exists")
    }

    /// CF for storing blocks at a fixed global slot:
    /// `global slot -> list of blocks at slot`
    ///
    /// - `list of blocks at slot`: sorted from best to worst
    fn slots_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-at-slot")
            .expect("blocks-at-slot column family exists")
    }

    fn canonicity_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("canonicity")
            .expect("canonicity column family exists")
    }

    fn user_commands_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("user-commands")
            .expect("user-commands column family exists")
    }

    fn internal_commands_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("mainnet-internal-commands")
            .expect("mainnet-internal-commands column family exists")
    }

    fn internal_commands_slot_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("internal-commands-global-slot-idx")
            .expect("internal-commands-global-slot-idx column family exists")
    }

    /// CF for sorting user commands: `{global_slot}{txn_hash} -> data`
    ///
    /// - `global_slot`: 4 BE bytes
    fn commands_slot_mainnet_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("mainnet-commands-slot")
            .expect("mainnet-commands-slot column family exists")
    }

    /// CF for storing: `txn_hash -> global_slot`
    ///
    /// - `global_slot`: 4 BE bytes
    fn commands_txn_hash_to_global_slot_mainnet_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("mainnet-cmds-txn-global-slot")
            .expect("mainnet-cmds-txn-global-slot column family exists")
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

    /// CF for storing all snark work fee totals
    fn snark_top_producers_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snark-work-top-producers")
            .expect("snark-work-top-producers column family exists")
    }

    /// CF for sorting all snark work fee totals
    fn snark_top_producers_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snark-work-top-producers-sort")
            .expect("snark-work-top-producers-sort column family exists")
    }

    /// CF for storing/sorting SNARK work fees
    fn snark_work_fees_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snark-work-fees")
            .expect("snark-work-fees column family exists")
    }

    /// CF for storing chain_id -> network
    fn chain_id_to_network_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("chain-id-to-network")
            .expect("chain-id-to-network column family exists")
    }

    /// CF for sorting user commands by sender public key
    fn txn_from_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("txn-from")
            .expect("txn-from column family exists")
    }

    /// CF for sorting user commands by receiver public key in [CommandStore]
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
    /// - key: `pk`
    /// - value: total number of blocks produced by `pk`
    fn block_production_pk_total_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-production-pk-total")
            .expect("block-production-pk-total column family exists")
    }

    /// CF for per epoch block production totals
    /// - key: `epoch`
    /// - value: number of blocks produced in `epoch`
    fn block_production_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-production-epoch")
            .expect("block-production-epoch column family exists")
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
    fn username_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("usernames")
            .expect("usernames column family exists")
    }
}
