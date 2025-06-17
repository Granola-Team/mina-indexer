//! Column family helpers impl

use crate::store::{column_families::ColumnFamilyHelpers, IndexerStore};
use speedb::ColumnFamily;

impl ColumnFamilyHelpers for IndexerStore {
    /////////////////////
    // Block store CFs //
    /////////////////////

    /// CF for storing blocks
    /// ```
    /// key: [StateHash] bytes
    /// val: {num block bytes BE u64 bytes}{serde_json block bytes}
    fn blocks_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks")
            .expect("blocks column family exists")
    }

    /// CF for storing block state hashes
    /// ```
    /// key: [StateHash] bytes
    /// val: {num block bytes BE u64 bytes}{serde_json block bytes}
    fn blocks_state_hash_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-state-hash")
            .expect("blocks-state-hash column family exists")
    }

    /// CF for storing PCB versions
    /// ```
    /// key: [StateHash] bytes
    /// val: [PcbVersion] serde bytes
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
    /// - global_slot: [u32] BE bytes
    /// - state_hash:  [StateHash] bytes
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
    /// - block_height: [u32] BE bytes
    /// - state_hash:   [StateHash] bytes
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

    fn block_total_supply_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-total-supply")
            .expect("blocks-total-supply column family exists")
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

    fn block_creator_height_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-creator-height-sort")
            .expect("block-creator-height-sort column family exists")
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

    /// CF for storing per epoch slots produced
    /// ```
    /// key: {genesis}{epoch}{slot}
    /// val: b""
    /// where
    /// - genesis: [StateHash] bytes
    /// - epoch:   [u32] BE bytes
    /// - slot:    [u32] BE bytes
    /// ```
    /// Use [epoch_num_key]
    fn block_epoch_slots_produced_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-epoch-slots-produced")
            .expect("block-epoch-slots-produced column family exists")
    }

    /// CF for storing per epoch per account slots produced
    /// ```
    /// key: {genesis}{epoch}{slot}
    /// val: b""
    /// where
    /// - genesis: [StateHash] bytes
    /// - epoch:   [u32] BE bytes
    /// - pk:      [PublicKey] bytes
    /// - slot:    [u32] BE bytes
    /// ```
    /// Use [epoch_pk_num_key]
    fn block_pk_epoch_slots_produced_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-pk-epoch-slots-produced")
            .expect("block-pk-epoch-slots-produced column family exists")
    }

    /// CF for storing the number of blocks for a specified public key
    /// ```
    /// key: [PublicKey] bytes
    /// val: [u32] BE bytes
    fn blocks_pk_count_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-pk-count")
            .expect("blocks-pk-count column family exists")
    }

    /// CF for storing the tokens used in a blocks
    /// ```
    /// key: [StateHash] bytes
    /// val: [PublicKey] bytes
    fn blocks_tokens_used_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("blocks-tokens-used")
            .expect("blocks-tokens-used column family exists")
    }

    ////////////////////////////
    // User command store CFs //
    ////////////////////////////

    /// Key-value pairs
    /// ```
    /// - key: {txn_hash}{state_hash}
    /// - value: [SignedCommandWithData] serde bytes
    /// where
    /// - txn_hash:   [TxnHash::V1_LEN] bytes (v2 is right-padded)
    /// - state_hash: [StateHash] bytes
    fn user_commands_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("user-commands")
            .expect("user-commands column family exists")
    }

    /// Key-value pairs
    /// ```
    /// - key: [TxnHash] bytes
    /// - value: [Vec<StateHash>] serde bytes (sorted)
    fn user_commands_state_hashes_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("user-commands-state-hashes")
            .expect("user-commands-state-hashes column family exists")
    }

    fn user_commands_per_block_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("user-commands-block")
            .expect("user-commands-block column family exists")
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
    /// - val: [SignedCommandWithData] serde bytes
    /// where
    /// - height:     [u32] BE bytes
    /// - txn_hash:   [TxnHash::V1_LEN] bytes
    /// - state_hash: [StateHash] bytes
    /// ```
    /// Use with [txn_sort_key]
    fn user_commands_height_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("user-commands-height-sort")
            .expect("user-commands-height-sort column family exists")
    }

    /// Key-value pairs
    /// ```
    /// - key: {token}{height}{txn_hash}{state_hash}
    /// - val: [SignedCommandWithData] serde bytes
    /// where
    /// - height:     [u32] BE bytes
    /// - txn_hash:   [TxnHash::V1_LEN] bytes
    /// - state_hash: [StateHash] bytes
    /// ```
    /// Use with [token_txn_sort_key]
    fn user_commands_per_token_height_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("user-commands-per-token-height-sort")
            .expect("user-commands-per-token-height-sort column family exists")
    }

    /// Key-value pairs
    /// ```
    /// - key: txn_hash
    /// - val: block_height
    /// where
    /// - txn_hash:     [TxnHash::V1_LEN] bytes
    /// - block_height: [u32] BE bytes
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
    /// - txn_hash:    [TxnHash::V1_LEN] bytes
    /// - global_slot: [u32] BE bytes
    fn user_commands_txn_hash_to_global_slot_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("user-commands-to-global-slot")
            .expect("user-commands-to-global-slot column family exists")
    }

    /// Key-value pairs
    /// ```
    /// - key: {sender}{block_height}{nonce}{txn_hash}{state_hash}
    /// - val: amount
    /// where
    /// - sender:       [PublicKey] bytes
    /// - block_height: [u32] BE bytes
    /// - nonce:        [u32] BE bytes
    /// - txn_hash:     [TxnHash::V1_LEN] bytes
    /// - state_hash:   [StateHash] bytes
    /// - amount:       [u64] BE bytes
    /// ```
    /// Use with [pk_txn_sort_key]
    fn txn_from_height_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("txn-from-height-sort")
            .expect("txn-from-height-sort column family exists")
    }

    /// Key-value pairs
    /// ```
    /// - key: {receiver}{height}{nonce}{txn_hash}{state_hash}
    /// - val: amount
    /// where
    /// - receiver:   [PublicKey] bytes
    /// - height:     [u32] BE bytes
    /// - nonce:      [u32] BE bytes
    /// - txn_hash:   [TxnHash::V1_LEN] bytes
    /// - state_hash: [StateHash] bytes
    /// - amount:     [u64] BE bytes
    /// ```
    /// Use with [pk_txn_sort_key]
    fn txn_to_height_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("txn-to-height-sort")
            .expect("txn-to-height-sort column family exists")
    }

    /// Key-value pairs
    /// ```
    /// - key: {txn_hash}{state_hash}
    /// - val: [SignedCommandWithData] serde bytes
    /// where
    /// - txn_hash:   [TxnHash::V1_LEN] bytes (v2 is right-padded)
    /// - state_hash: [StateHash] bytes
    fn zkapp_commands_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-commands")
            .expect("zkapp-commands column family exists")
    }

    fn zkapp_commands_pk_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-commands-pk")
            .expect("zkapp-commands-pk column family exists")
    }

    fn zkapp_commands_pk_num_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-commands-pk-num")
            .expect("zkapp-commands-pk-num column family exists")
    }

    /// Key-value pairs
    /// ```
    /// - key: {height}{txn_hash}{state_hash}
    /// - val: [SignedCommandWithData] serde bytes
    /// where
    /// - height:     [u32] BE bytes
    /// - txn_hash:   [TxnHash::V1_LEN] bytes (right-padded)
    /// - state_hash: [StateHash] bytes
    fn zkapp_commands_height_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-commands-height-sort")
            .expect("zkapp-commands-height-sort column family exists")
    }

    /////////////////////
    // Zkapp store CFs //
    /////////////////////

    /// #### CF for storing zkapp states
    ///
    /// Key-value pairs
    /// ```
    /// key: {token}{pk}{num}
    /// val: [[AppState; ZKAPP_STATE_FIELD_ELEMENTS_NUM]] serde bytes
    /// where:
    /// - token: [TokenAddress] bytes
    /// - pk:    [PublicKey] bytes
    /// - num:   [u32] BE bytes
    fn zkapp_state_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-state")
            .expect("zkapp-state column family exists")
    }

    /// #### CF for storing zkapp state counts
    ///
    /// Key-value pairs
    /// ```
    /// key: {token}{pk}
    /// val: [u32] BE bytes
    /// where:
    /// - token: [TokenAddress] bytes
    /// - pk:    [PublicKey] bytes
    fn zkapp_state_num_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-state-num")
            .expect("zkapp-state-num column family exists")
    }

    /// #### CF for storing zkapp permissions
    ///
    /// Key-value pairs
    /// ```
    /// key: {token}{pk}{num}
    /// val: [Permissions] serde bytes
    /// where:
    /// - token: [TokenAddress] bytes
    /// - pk:    [PublicKey] bytes
    /// - num:   [u32] BE bytes
    fn zkapp_permissions_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-permissions")
            .expect("zkapp-permissions column family exists")
    }

    /// #### CF for storing zkapp permission counts
    ///
    /// Key-value pairs
    /// ```
    /// key: {token}{pk}
    /// val: [u32] BE bytes
    /// where:
    /// - token: [TokenAddress] bytes
    /// - pk:    [PublicKey] bytes
    fn zkapp_permissions_num_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-permissions-num")
            .expect("zkapp-permissions-num column family exists")
    }

    /// #### CF for storing zkapp verification keys
    ///
    /// Key-value pairs
    /// ```
    /// key: {token}{pk}{num}
    /// val: [VerificationKey] serde bytes
    /// where:
    /// - token: [TokenAddress] bytes
    /// - pk:    [PublicKey] bytes
    /// - num:   [u32] BE bytes
    fn zkapp_verification_key_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-verification-key")
            .expect("zkapp-verification-key column family exists")
    }

    /// #### CF for storing zkapp verification key counts
    ///
    /// Key-value pairs
    /// ```
    /// key: {token}{pk}
    /// val: [u32] BE bytes
    /// where:
    /// - token: [TokenAddress] bytes
    /// - pk:    [PublicKey] bytes
    fn zkapp_verification_key_num_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-verification-key-num")
            .expect("zkapp-verification-key-num column family exists")
    }

    /// #### CF for storing zkapp uris
    ///
    /// Key-value pairs
    /// ```
    /// key: {token}{pk}{num}
    /// val: [ZkappUri] bytes
    /// where:
    /// - token: [TokenAddress] bytes
    /// - pk:    [PublicKey] bytes
    /// - num:   [u32] BE bytes
    fn zkapp_uri_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-uri")
            .expect("zkapp-uri column family exists")
    }

    /// #### CF for storing zkapp uri counts
    ///
    /// Key-value pairs
    /// ```
    /// key: {token}{pk}
    /// val: [u32] BE bytes
    /// where:
    /// - token: [TokenAddress] bytes
    /// - pk:    [PublicKey] bytes
    fn zkapp_uri_num_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-uri-num")
            .expect("zkapp-uri-num column family exists")
    }

    /// #### CF for storing zkapp token symbols
    ///
    /// Key-value pairs
    /// ```
    /// key: {token}{pk}{num}
    /// val: [TokenSymbol] bytes
    /// where:
    /// - token: [TokenAddress] bytes
    /// - pk:    [PublicKey] bytes
    /// - num:   [u32] BE bytes
    fn zkapp_token_symbol_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-token-symbol")
            .expect("zkapp-token-symbol column family exists")
    }

    /// #### CF for storing zkapp token symbol counts
    ///
    /// Key-value pairs
    /// ```
    /// key: {token}{pk}
    /// val: [u32] BE bytes
    /// where:
    /// - token: [TokenAddress] bytes
    /// - pk:    [PublicKey] bytes
    fn zkapp_token_symbol_num_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-token-symbol-num")
            .expect("zkapp-token-symbol-num column family exists")
    }

    /// #### CF for storing zkapp timings
    ///
    /// Key-value pairs
    /// ```
    /// key: {token}{pk}{num}
    /// val: [Timing] serde bytes
    /// where:
    /// - token: [TokenAddress] bytes
    /// - pk:    [PublicKey] bytes
    /// - num:   [u32] BE bytes
    fn zkapp_timing_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-timing")
            .expect("zkapp-timing column family exists")
    }

    /// #### CF for storing zkapp timing counts
    ///
    /// Key-value pairs
    /// ```
    /// key: {token}{pk}
    /// val: [u32] BE bytes
    /// where:
    /// - token: [TokenAddress] bytes
    /// - pk:    [PublicKey] bytes
    fn zkapp_timing_num_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-timing-num")
            .expect("zkapp-timing-num column family exists")
    }

    /// #### CF for storing zkapp actions
    ///
    /// Key-value pairs
    /// ```
    /// key: {token}{pk}{num}
    /// val: [Vec<ActionState>] serde bytes
    /// where:
    /// - token: [TokenAddress] bytes
    /// - pk:    [PublicKey] bytes
    /// - num:   [u32] BE bytes
    fn zkapp_actions_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-actions")
            .expect("zkapp-actions column family exists")
    }

    /// #### CF for storing a zkapp account's current action count
    ///
    /// Key-value pairs
    /// ```
    /// key: {token}{pk}
    /// val: [u32] BE bytes
    /// where:
    /// - token: [TokenAddress] bytes
    /// - pk:    [PublicKey] bytes
    fn zkapp_actions_pk_num_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-actions-pk-num")
            .expect("zkapp-actions-pk-num column family exists")
    }

    /// #### CF for storing zkapp events
    ///
    /// Key-value pairs
    /// ```
    /// key: {token}{pk}{num}
    /// val: [Vec<ZkappEvent>] serde bytes
    /// where:
    /// - token: [TokenAddress] bytes
    /// - pk:    [PublicKey] bytes
    /// - num:   [u32] BE bytes
    fn zkapp_events_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-events")
            .expect("zkapp-events column family exists")
    }

    /// #### CF for storing a zkapp account's current event count
    ///
    /// Key-value pairs
    /// ```
    /// key: {token}{pk}
    /// val: [u32] BE bytes
    /// where:
    /// - token: [TokenAddress] bytes
    /// - pk:    [PublicKey] bytes
    fn zkapp_events_pk_num_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-events-pk-num")
            .expect("zkapp-events-pk-num column family exists")
    }

    /// #### CF for storing tokens
    ///
    /// Key-value pairs
    /// ```
    /// - key: [TokenAddress] bytes
    /// - val: [Token] serde bytes
    fn zkapp_tokens_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-tokens")
            .expect("zkapp-tokens column family exists")
    }

    /// #### CF for sorting tokens by supply
    ///
    /// Key-value pairs
    /// ```
    /// - key: {token}{supply}
    /// - val: [Token] serde bytes
    /// where
    /// - token:  [TokenAddress] bytes
    /// - supply: [u64] BE bytes
    fn zkapp_tokens_supply_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-tokens-supply-sort")
            .expect("zkapp-tokens-supply-sort column family exists")
    }

    /// #### CF for storing tokens at indexes
    ///
    /// Key-value pairs
    /// ```
    /// - key: [u32] BE bytes
    /// - val: [Token] serde bytes
    fn zkapp_tokens_at_index_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-tokens-at-index")
            .expect("zkapp-tokens-at-index column family exists")
    }

    /// #### CF for storing token indexes
    ///
    /// Key-value pairs
    /// ```
    /// - key: [TokenAddress] bytes
    /// - val: [u32] BE bytes
    fn zkapp_tokens_index_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-tokens-index")
            .expect("zkapp-tokens-index column family exists")
    }

    /// #### CF for storing token supplies
    ///
    /// Key-value pairs
    /// ```
    /// - key: [TokenAddress] bytes
    /// - val: [u32] BE bytes
    fn zkapp_tokens_supply_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-tokens-supply")
            .expect("zkapp-tokens-supply column family exists")
    }

    /// #### CF for storing token owners
    ///
    /// Key-value pairs
    /// ```
    /// - key: [TokenAddress] bytes
    /// - val: [PublicKey] serde bytes
    fn zkapp_tokens_owner_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-tokens-owner")
            .expect("zkapp-tokens-owner column family exists")
    }

    /// #### CF for storing token symbols
    ///
    /// Key-value pairs
    /// ```
    /// - key: [TokenAddress] bytes
    /// - val: [TokenSymbol] serde bytes
    fn zkapp_tokens_symbol_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-tokens-symbol")
            .expect("zkapp-tokens-symbol column family exists")
    }

    /// #### CF for storing holders per token
    ///
    /// Key-value pairs
    /// ```
    /// - key: {token}{index}
    /// - val: [Account] serde bytes
    /// where
    /// - token: [TokenAddress] bytes
    /// - index: [u32] BE bytes
    fn zkapp_tokens_holder_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-tokens-holder")
            .expect("zkapp-tokens-holder column family exists")
    }

    /// #### CF for storing token holder counts
    ///
    /// Key-value pairs
    /// ```
    /// - key: [TokenAddress] bytes
    /// - val: [u32] BE bytes
    fn zkapp_tokens_holder_count_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-tokens-holder-count")
            .expect("zkapp-tokens-holder-count column family exists")
    }

    /// #### CF for storing token holder indexes
    ///
    /// Key-value pairs
    /// ```
    /// - key: {token}{pk}
    /// - val: [u32] BE bytes
    /// where
    /// - token: [TokenAddress] bytes
    /// - pk:    [PublicKey] bytes
    fn zkapp_tokens_holder_index_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-tokens-holder-index")
            .expect("zkapp-tokens-holder-index column family exists")
    }

    /// #### CF for storing tokens per holder
    ///
    /// Key-value pairs
    /// ```
    /// - key: {pk}{index}
    /// - val: [Account] serde bytes
    /// where
    /// - pk:    [PublicKey] bytes
    /// - index: [u32] BE bytes
    /// ```
    /// Use with [zkapp_tokens_pk_key]
    fn zkapp_tokens_pk_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-tokens-pk")
            .expect("zkapp-tokens-pk column family exists")
    }

    /// #### CF for storing token counts per holder
    ///
    /// Key-value pairs
    /// ```
    /// - key: [PublicKey] bytes
    /// - val: [u32] BE bytes
    fn zkapp_tokens_pk_num_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-tokens-pk-num")
            .expect("zkapp-tokens-pk-num column family exists")
    }

    /// #### CF for storing token indexes per holder
    ///
    /// Key-value pairs
    /// ```
    /// - key: {token}{pk}
    /// - val: [u32] BE bytes
    /// where
    /// - token: [TokenAdress] bytes
    /// - pk:    [PublicKey] bytes
    /// ```
    /// Use with [zkapp_tokens_pk_index_key]
    fn zkapp_tokens_pk_index_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-tokens-pk-index")
            .expect("zkapp-tokens-pk-index column family exists")
    }

    /// #### CF for storing token transaction counts per token
    ///
    /// Key-value pairs
    /// ```
    /// - key: [TokenAddress] bytes
    /// - val: [u32] BE bytes
    fn zkapp_tokens_txns_num_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-tokens-txns-num")
            .expect("zkapp-tokens-txns-num column family exists")
    }

    /// #### CF for storing historical token diffs
    ///
    /// Key-value pairs
    /// ```
    /// - key: {token}{index}
    /// - val: [TokenDiff] serde bytes
    /// where
    /// - token: [TokenAdress] bytes
    /// - index: [u32] BE bytes
    /// ```
    /// Use with [zkapp_tokens_historical_diffs_key]
    fn zkapp_tokens_historical_diffs_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-tokens-historical-diffs")
            .expect("zkapp-tokens-historical-diffs column family exists")
    }

    /// #### CF for storing the count of historical token diffs
    ///
    /// Key-value pairs
    /// ```
    /// - key: [TokenAdress] bytes
    /// - val: [u32] BE bytes
    fn zkapp_tokens_historical_diffs_num_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-tokens-historical-diffs-num")
            .expect("zkapp-tokens-historical-diffs-num column family exists")
    }

    /// #### CF for storing historical token owners
    ///
    /// Key-value pairs
    /// ```
    /// - key: {token}{index}
    /// - val: [PublicKey] bytes
    /// where
    /// - token: [TokenAdress] bytes
    /// - index: [u32] BE bytes
    /// ```
    /// Use with [zkapp_tokens_historical_owners_key]
    fn zkapp_tokens_historical_owners_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-tokens-historical-owners")
            .expect("zkapp-tokens-historical-owners column family exists")
    }

    /// #### CF for storing the count of historical token owners
    ///
    /// Key-value pairs
    /// ```
    /// - key: [TokenAdress] bytes
    /// - val: [u32] BE bytes
    fn zkapp_tokens_historical_owners_num_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-tokens-historical-owners-num")
            .expect("zkapp-tokens-historical-owners-num column family exists")
    }

    /// #### CF for storing historical token symbols
    ///
    /// Key-value pairs
    /// ```
    /// - key: {token}{index}
    /// - val: [TokenSymbol] serde bytes
    /// where
    /// - token: [TokenAdress] bytes
    /// - index: [u32] BE bytes
    /// ```
    /// Use with [zkapp_tokens_historical_symbols_key]
    fn zkapp_tokens_historical_symbols_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-tokens-historical-symbols")
            .expect("zkapp-tokens-historical-symbols column family exists")
    }

    /// #### CF for storing the count of historical token symbols
    ///
    /// Key-value pairs
    /// ```
    /// - key: [TokenAdress] bytes
    /// - val: [u32] BE bytes
    fn zkapp_tokens_historical_symbols_num_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-tokens-historical-symbols-num")
            .expect("zkapp-tokens-historical-symbols-num column family exists")
    }

    /// #### CF for storing historical token supplies
    ///
    /// Key-value pairs
    /// ```
    /// - key: {token}{index}
    /// - val: [Amount] serde bytes
    /// where
    /// - token: [TokenAdress] bytes
    /// - index: [u32] BE bytes
    /// ```
    /// Use with [zkapp_tokens_historical_supplies_key]
    fn zkapp_tokens_historical_supplies_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-tokens-historical-supplies")
            .expect("zkapp-tokens-historical-supplies column family exists")
    }

    /// #### CF for storing the count of historical token supplies
    ///
    /// Key-value pairs
    /// ```
    /// - key: [TokenAdress] bytes
    /// - val: [u32] BE bytes
    fn zkapp_tokens_historical_supplies_num_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-tokens-historical-supplies-num")
            .expect("zkapp-tokens-historical-supplies-num column family exists")
    }

    /// #### CF for storing historical pk token diffs
    ///
    /// Key-value pairs
    /// ```
    /// - key: {pk}{index}
    /// - val: [TokenDiff] serde bytes
    /// where
    /// - pk:    [PublicKey] bytes
    /// - index: [u32] BE bytes
    fn zkapp_tokens_historical_pk_diffs_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-tokens-historical-pk-diffs")
            .expect("zkapp-tokens-historical-pk-diffs column family exists")
    }

    /// #### CF for storing the count of historical pk token token diffs
    ///
    /// Key-value pairs
    /// ```
    /// - key: [PublicKey] bytes
    /// - val: [u32] BE bytes
    /// ```
    /// Use with [zkapp_tokens_historical_pk_diffs_key]
    fn zkapp_tokens_historical_pk_diffs_num_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-tokens-historical-pk-diffs-num")
            .expect("zkapp-tokens-historical-pk-diffs-num column family exists")
    }

    ////////////////////////////////
    // Internal command store CFs //
    ////////////////////////////////

    /// Key-value pairs
    /// ```
    /// - key: {state_hash}{index}
    /// - val: [InternalCommandWithData] serde bytes
    /// where
    /// - state_hash: [StateHash] bytes
    /// - index:      [u32] BE bytes
    /// ```
    /// Use with [internal_commmand_block_key]
    fn internal_commands_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("internal-commands")
            .expect("internal-commands column family exists")
    }

    /// Key-value pairs
    /// ```
    /// - key: [StateHash] bytes
    /// - val: [u32] BE bytes
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
    /// - receiver: [PublicKey] bytes
    /// - index:    [u32] BE bytes
    /// ```
    /// Use with [internal_command_pk_key]
    fn internal_commands_pk_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("internal-commands-pk")
            .expect("internal-commands-pk column family exists")
    }

    /// Key-value pairs
    /// ```
    /// - key: [PublicKey] bytes
    /// - val: [u32] BE bytes
    fn internal_commands_pk_num_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("internal-commands-pk-num")
            .expect("internal-commands-pk-num column family exists")
    }

    /// Key-value pairs
    /// ```
    /// - key: {height}{state_hash}{index}{kind}
    /// - val: [InternalCommandWithData] serde bytes
    /// where
    /// - height:     [u32] BE bytes
    /// - state_hash: [StateHash] bytes
    /// - index:      [u32] BE bytes
    /// - kind:       0, 1, or 2
    /// ```
    /// Use with [internal_commmand_sort_key]
    fn internal_commands_block_height_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("internal-commands-block-height-sort")
            .expect("internal-commands-block-height-sort column family exists")
    }

    /// Key-value pairs
    /// ```
    /// - key: {recipient}{height}{state_hash}{index}
    /// - val: [InternalCommandWithData] serde bytes
    /// where
    /// - recipient:  [PublicKey] bytes
    /// - height:     [u32] BE bytes
    /// - state_hash: [StateHash] bytes
    /// - index:      [u32] BE bytes
    /// ```
    /// Use with [internal_commmand_pk_sort_key]
    fn internal_commands_pk_block_height_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("internal-commands-pk-block-height-sort")
            .expect("internal-commands-pk-block-height-sort column family exists")
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
    /// key: {token}{pk}
    /// val: [Account] serde bytes
    /// where
    /// - token:   [TokenAddress] bytes
    /// - pk:      [PublicKey] bytes
    /// ```
    /// Use [best_account_key]
    fn best_ledger_accounts_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("best-ledger-accounts")
            .expect("best-ledger-accounts column family exists")
    }

    /// CF for sorting best ledger accounts by balance
    /// ```
    /// key: {token}{balance}{pk}
    /// val: [Account] serde bytes
    /// where
    /// - token:   [TokenAddress] bytes
    /// - balance: [u64] BE bytes
    /// - pk:      [PublicKey] bytes
    /// ```
    /// Use with [best_account_sort_key]
    fn best_ledger_accounts_balance_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("best-ledger-account-balance-sort")
            .expect("best-ledger-account-balance-sort column family exists")
    }

    /// CF for storing zkapp best ledger accounts
    /// ```
    /// key: {token}{pk}
    /// val: [Account] serde bytes
    /// where
    /// - token:   [TokenAddress] bytes
    /// - pk:      [PublicKey] bytes
    /// ```
    /// Use [best_account_key]
    fn zkapp_best_ledger_accounts_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-best-ledger-accounts")
            .expect("zkapp-best-ledger-accounts column family exists")
    }

    /// CF for sorting zkapp best ledger accounts by balance
    /// ```
    /// key: {token}{balance}{pk}
    /// val: [Account] serde bytes
    /// where
    /// - token:   [TokenAddress] bytes
    /// - balance: [u64] BE bytes
    /// - pk:      [PublicKey] bytes
    fn zkapp_best_ledger_accounts_balance_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-best-ledger-account-balance-sort")
            .expect("zkapp-best-ledger-account-balance-sort column family exists")
    }

    /// CF for storing number of best ledger account delegations
    /// ```
    /// pk -> num
    /// where
    /// - pk:  [PublicKey] bytes
    /// - num: [u32] BE bytes
    fn best_ledger_accounts_num_delegations_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("best-ledger-account-num-delegations")
            .expect("best-ledger-account-num-delegations column family exists")
    }

    /// CF for storing best ledger account delegations
    /// ```
    /// {pk}{num} -> delegate
    /// where
    /// - pk:  [PublicKey] bytes
    /// - num: [u32] BE bytes
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
    /// {state_hash}{token}{pk} -> account
    /// where
    /// - state_hash: [StateHash] bytes
    /// - token:      [TokenAddress] bytes
    /// - pk:         [PublicKey] bytes
    /// - account:    [Account] serde bytes
    fn staged_ledger_accounts_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staged-ledger-accounts")
            .expect("staged-ledger-accounts column family exists")
    }

    /// CF for sorting staged ledger accounts by balance
    /// ```
    /// {state_hash}{token}{balance}{pk} -> _
    /// where
    /// - state_hash: [StateHash] bytes
    /// - token:      [TokenAddress] bytes
    /// - balance:    [u64] BE bytes
    /// - pk:         [PublicKey] bytes
    fn staged_ledger_account_balance_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staged-ledger-account-balance-sort")
            .expect("staged-ledger-account-balance-sort column family exists")
    }

    /// CF for storing number of staged ledger account delegations
    /// ```
    /// {state_hash}{pk} -> num
    /// where
    /// - state_hash: [StateHash] bytes
    /// - pk:         [PublicKey] bytes
    /// - num:        [u32] BE bytes
    fn staged_ledger_account_num_delegations_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staged-ledger-account-num-delegations")
            .expect("staged-ledger-account-num-delegations column family exists")
    }

    /// CF for storing staged ledger account delegations
    /// ```
    /// {state_hash}{pk}{num} -> delegate
    /// where
    /// - state_hash: [StateHash] bytes
    /// - pk:         [PublicKey] bytes
    /// - num:        [u32] BE bytes
    /// - delegate:   [PublicKey] bytes
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
    /// ```
    /// key: [PublicKey] bytes
    /// val: [StateHashWithHeight] serde bytes
    fn staged_ledger_accounts_min_block_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staged-ledger-accounts-min-block")
            .expect("staged-ledger-accounts-min-block column family exists")
    }

    /// CF for storing block ledger diffs
    /// ```
    /// key: [StateHash] bytes
    /// val: [LedgerDiff] serde bytes
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
    /// key: [staking_ledger_account_key]
    /// val: [Account] serde bytes
    fn staking_ledger_accounts_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staking-ledger-accounts")
            .expect("staking-ledger-accounts column family exists")
    }

    /// CF for storing aggregated staking delegations
    /// ```
    /// key: [staking_ledger_account_key]
    /// val: [StakingAccountWithEpochDelegation] serde bytes
    fn staking_delegations_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staking-ledger-delegations")
            .expect("staking-ledger-delegations column family exists")
    }

    /// CF for storing aggregated staking delegations
    /// ```
    /// key: [staking_ledger_epoch_key]
    /// val: b""
    fn staking_ledger_persisted_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staking-ledger-persisted")
            .expect("staking-ledger-persisted column family exists")
    }

    /// CF for storing staking ledger hashes
    /// ```
    /// key: [staking_ledger_epoch_key_prefix]
    /// val: [LedgerHash] bytes
    fn staking_ledger_epoch_to_hash_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staking-ledger-epoch-to-hash")
            .expect("staking-ledger-epoch-to-hash column family exists")
    }

    /// CF for storing staking ledger epochs
    /// ```
    /// key: [LedgerHash] bytes
    /// val: [u32] BE bytes
    fn staking_ledger_hash_to_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staking-ledger-hash-to-epoch")
            .expect("staking-ledger-hash-to-epoch column family exists")
    }

    /// CF for storing staking ledger genesis state hashes
    /// ```
    /// key: [LedgerHash] bytes
    /// val: [StateHash] bytes
    fn staking_ledger_genesis_hash_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staking-ledger-genesis-hash")
            .expect("staking-ledger-genesis-hash column family exists")
    }

    /// CF for storing staking ledger total currencies
    /// ```
    /// key: [LedgerHash] bytes
    /// val: [u64] BE bytes
    fn staking_ledger_total_currency_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staking-ledger-total-currency")
            .expect("staking-ledger-total-currency column family exists")
    }

    /// CF for sorting staking ledger accounts by balance
    /// ```
    /// key: [staking_ledger_sort_key]
    /// val: [StakingAccountWithEpochDelegation] serde bytes
    fn staking_ledger_balance_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staking-ledger-balance-sort")
            .expect("staking-ledger-balance-sort column family exists")
    }

    /// CF for sorting staking ledger accounts by stake (i.e. total delegations)
    /// ```
    /// key: [staking_ledger_sort_key]
    /// val: [StakingAccountWithEpochDelegation] serde bytes
    fn staking_ledger_stake_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staking-ledger-stake-sort")
            .expect("staking-ledger-stake-sort column family exists")
    }

    /// CF for storing per epoch total number of staking ledger accounts
    /// ```
    /// key: {genesis}{epoch}
    /// val: [u32] BE bytes
    /// where
    /// - genesis: [StateHash] bytes
    /// - epoch:   [u32] BE bytes
    fn staking_ledger_accounts_count_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("staking-ledger-accounts-count-epoch")
            .expect("staking-ledger-accounts-count-epoch column family exists")
    }

    /////////////////////
    // SNARK store CFs //
    /////////////////////

    /// CF for storing SNARKs by block state hash
    /// ```
    /// key: {hash}{index}
    /// val: snark
    /// where
    /// - hash:  [StateHash] bytes
    /// - index: [u32] BE bytes
    /// - snark: [SnarkWorkSummary] serde bytes
    /// ```
    /// Use [block_index_key]
    fn snarks_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snarks")
            .expect("snarks column family exists")
    }

    /// CF for storing SNARKs by prover
    /// ```
    /// key: {prover}{index}
    /// val: snark
    /// where
    /// - prover: [PublicKey] bytes
    /// - index:  [u32] BE bytes
    /// - snark:  [SnarkWorkSummaryWithStateHash] serde bytes
    /// ```
    /// Use [pk_index_key]
    fn snarks_prover_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snarks-prover")
            .expect("snarks-prover column family exists")
    }

    /// CF for storing SNARK total fees by prover
    /// ```
    /// key: [PublicKey] bytes
    /// val: [u64] BE bytes
    fn snark_prover_fees_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snark-prover-fees")
            .expect("snark-prover-fees column family exists")
    }

    /// CF for storing per epoch per prover SNARK total fees
    /// ```
    /// key: {genesis}{epoch}{prover}
    /// val: [u64] BE bytes
    /// where
    /// - genesis: [StateHash] bytes
    /// - epoch:   [u32] BE bytes
    /// - prover:  [PublicKey] bytes
    /// ```
    /// Use [snarks_pk_epoch_key]
    fn snark_prover_fees_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snark-prover-fees-epoch")
            .expect("snark-prover-fees-epoch column family exists")
    }

    /// CF for storing historical SNARK all-time fee updates
    /// ```
    /// key: {prover}{height}
    /// val: [SnarkAllTimeFees] serde bytes
    /// where
    /// - prover: [PublicKey] bytes
    /// - height: [u32] BE bytes
    /// ```
    /// Use [pk_index_key]
    fn snark_prover_fees_historical_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snark-prover-fees-historical")
            .expect("snark-prover-fees-historical column family exists")
    }

    /// CF for storing historical SNARK epoch fee updates
    /// ```
    /// key: {genesis}{epoch}{prover}{height}
    /// val: [SnarkEpochFees] serde bytes
    /// where
    /// - genesis: [StateHash] bytes
    /// - epoch:   [u32] BE bytes
    /// - prover:  [PublicKey] bytes
    /// - height:  [u32] BE bytes
    /// ```
    /// Use [snarks_epoch_pk_index_key]
    fn snark_prover_fees_epoch_historical_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snark-prover-fees-epoch-historical")
            .expect("snark-prover-fees-epoch-historical column family exists")
    }

    /// CF for sorting SNARK provers by total fees
    /// ```
    /// key: {fees}{prover}
    /// val: b""
    /// where
    /// - fees:   [u64] BE bytes
    /// - prover: [PublicKey] bytes
    /// ```
    /// Use [u64_prefix_key]
    fn snark_prover_total_fees_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snark-prover-total-fees-sort")
            .expect("snark-prover-total-fees-sort column family exists")
    }

    /// CF for sorting per epoch SNARK provers by total fees
    /// ```
    /// key: {genesis}{epoch}{fees}{prover}
    /// val: b""
    /// where
    /// - genesis: [StateHash] bytes
    /// - epoch:   [u32] BE bytes
    /// - fees:    [u64] BE bytes
    /// - prover:  [PublicKey] bytes
    /// ```
    /// Use [snark_fee_epoch_sort_key]
    fn snark_prover_total_fees_epoch_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snark-prover-total-fees-epoch-sort")
            .expect("snark-prover-total-fees-epoch-sort column family exists")
    }

    /// CF for storing SNARK prover max fees
    /// ```
    /// key: [PublicKey] bytes
    /// val: [u64] BE bytes
    fn snark_prover_max_fee_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snark-prover-max-fee")
            .expect("snark-prover-max-fee column family exists")
    }

    /// CF for storing per epoch SNARK prover max fees
    /// ```
    /// key: {genesis}{epoch}{prover}
    /// val: [u64] BE bytes
    /// where
    /// - genesis: [StateHash] bytes
    /// - epoch:   [u32] BE bytes
    /// - prover:  [PublicKey] bytes
    /// ```
    /// Use [snarks_pk_epoch_key]
    fn snark_prover_max_fee_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snark-prover-max-fee-epoch")
            .expect("snark-prover-max-fee-epoch column family exists")
    }

    /// CF for sorting SNARK provers by max fee
    /// ```
    /// key: {fee}{prover}
    /// val: b""
    /// where
    /// - fee:    [u64] BE bytes
    /// - prover: [PublicKey] bytes
    /// ```
    /// Use [u64_prefix_key]
    fn snark_prover_max_fee_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snark-prover-max-fee-sort")
            .expect("snark-prover-max-fee-sort column family exists")
    }

    /// CF for sorting per epoch SNARK provers by max fee
    /// ```
    /// key: {genesis}{epoch}{fee}{prover}
    /// val: b""
    /// where
    /// - genesis: [StateHash] bytes
    /// - epoch:   [u32] BE bytes
    /// - fee:     [u64] BE bytes
    /// - prover:  [PublicKey] bytes
    /// ```
    /// Use [snark_fee_epoch_sort_key]
    fn snark_prover_max_fee_epoch_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snark-prover-max-fee-epoch-sort")
            .expect("snark-prover-max-fee-epoch-sort column family exists")
    }

    /// CF for storing SNARK prover min fees
    /// ```
    /// key: [PublicKey] bytes
    /// val: [u64] BE bytes
    fn snark_prover_min_fee_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snark-prover-min-fee")
            .expect("snark-prover-min-fee column family exists")
    }

    /// CF for storing per epoch SNARK prover min fees
    /// ```
    /// key: {genesis}{epoch}{prover}
    /// val: [u64] BE bytes
    /// where
    /// - genesis: [StateHash] bytes
    /// - epoch:   [u32] BE bytes
    /// - prover:  [PublicKey] bytes
    /// ```
    /// Use [snarks_pk_epoch_key]
    fn snark_prover_min_fee_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snark-prover-min-fee-epoch")
            .expect("snark-prover-min-fee-epoch column family exists")
    }

    /// CF for sorting SNARK provers by min fee
    /// ```
    /// key: {fee}{prover}
    /// val: b""
    /// where
    /// - fee:    [u64] BE bytes
    /// - prover: [PublicKey] bytes
    /// ```
    /// Use [u64_prefix_key]
    fn snark_prover_min_fee_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snark-prover-min-fee-sort")
            .expect("snark-prover-min-fee-sort column family exists")
    }

    /// CF for sorting per epoch SNARK provers by min fee
    /// ```
    /// key: {genesis}{epoch}{fee}{prover}
    /// val: b""
    /// where
    /// - genesis: [StateHash] bytes
    /// - epoch:   [u32] BE bytes
    /// - fee:     [u64] BE bytes
    /// - prover:  [PublicKey] bytes
    /// ```
    /// Use [snark_fee_epoch_sort_key]
    fn snark_prover_min_fee_epoch_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snark-prover-min-fee-epoch-sort")
            .expect("snark-prover-min-fee-epoch-sort column family exists")
    }

    /// CF for storing/sorting SNARKs by prover & block height
    /// ```
    /// key: {prover}{block_height}{index}
    /// val: snark
    /// where
    /// - prover:       [PublicKey] bytes
    /// - block height: [u32] BE bytes
    /// - index:        [u32] BE bytes
    /// - snark:        [SnarkWorkSummary] serde bytes
    /// ```
    /// Use [snark_prover_sort_key]
    fn snark_prover_block_height_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snark-prover-block-height-sort")
            .expect("snark-prover-block-height-sort column family exists")
    }

    /// CF for sorting snark fees by block height
    /// ```
    /// key: {fee}{height}{pk}{hash}{index}
    /// val: b""
    /// where
    /// - fee:    [u64] BE bytes
    /// - height: [u32] BE bytes
    /// - pk:     [PublicKey] bytes
    /// - hash:   [StateHash] bytes
    /// - index:  [u32] BE bytes
    /// ```
    /// Use [snark_fee_sort_key]
    fn snark_work_fees_block_height_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snark-work-fees-block-height-sort")
            .expect("snark-work-fees-block-height-sort column family exists")
    }

    ////////////////////////
    // Username store CFs //
    ////////////////////////

    /// CF for storing username update count
    /// ```
    /// key: [PublicKey] bytes
    /// val: [u32] BE bytes
    fn username_num_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("username-num")
            .expect("username-num column family exists")
    }

    /// CF for storing usernames per pk
    /// ```
    /// key: {pk}{index}
    /// val: [Username] bytes
    /// where
    /// - pk:    [PublicKey] bytes
    /// - index: [u32] BE bytes
    fn username_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("username")
            .expect("username column family exists")
    }

    /// CF for storing usernames per block
    /// ```
    /// key: [StateHash] bytes
    /// val: [UsernameUpdate] serde bytes
    fn usernames_per_block_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("username-block")
            .expect("username-block column family exists")
    }

    /// CF for storing pk's per username
    /// ```
    /// key: [Username] bytes
    /// val: [BTreeSet<PublicKey>] serde bytes
    fn username_pk_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("username-pk")
            .expect("username-pk column family exists")
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

    /// CF for storing per epoch per account block prodution counts
    /// ```
    /// key: {genesis}{epoch}{pk}
    /// val: [u32] BE bytes
    /// where
    /// - genesis: [StateHash] bytes
    /// - epoch:   [u32] BE bytes
    /// - pk:      [PublicKey] bytes
    /// ```
    /// Use [epoch_pk_key]
    fn block_production_pk_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-production-pk-epoch")
            .expect("block-production-pk-epoch column family exists")
    }

    /// CF for storing per epoch per account canonical block production counts
    /// ```
    /// key: {genesis}{epoch}{pk}
    /// val: [u32] BE bytes
    /// where
    /// - genesis: [StateHash] bytes
    /// - epoch:   [u32] BE bytes
    /// - pk:      [PublicKey] bytes
    /// ```
    /// Use [epoch_pk_key]
    fn block_production_pk_canonical_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-production-pk-canonical-epoch")
            .expect("block-production-pk-canonical-epoch column family exists")
    }

    /// CF for sorting per epoch per account canonical block production counts
    /// ```
    /// key: {genesis}{epoch}{count}{pk}
    /// val: b""
    /// where
    /// - genesis: [StateHash] bytes
    /// - epoch:   [u32] BE bytes
    /// - count:   [u32] BE bytes
    /// - pk:      [PublicKey] bytes
    /// ```
    /// Use [epoch_pk_num_key]
    fn block_production_pk_canonical_epoch_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-production-pk-canonical-epoch-sort")
            .expect("block-production-pk-canonical-epoch-sort column family exists")
    }

    /// CF for storing per epoch per account supercharged block prodution counts
    /// ```
    /// key: {genesis}{epoch}{pk}
    /// val: [u32] BE bytes
    /// where
    /// - genesis: [StateHash] bytes
    /// - epoch:   [u32] BE bytes
    /// - pk:      [PublicKey] bytes
    /// ```
    /// Use [epoch_pk_key]
    fn block_production_pk_supercharged_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-production-pk-supercharged-epoch")
            .expect("block-production-pk-supercharged-epoch column family exists")
    }

    /// CF for storing per account total block prodution counts
    /// ```
    /// key: [PublicKey] bytes
    /// val: [u32] BE bytes
    fn block_production_pk_total_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-production-pk-total")
            .expect("block-production-pk-total column family exists")
    }

    /// CF for storing per account total canonical block prodution counts
    /// ```
    /// key: [PublicKey] bytes
    /// val: [u32] BE bytes
    fn block_production_pk_canonical_total_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-production-pk-canonical-total")
            .expect("block-production-pk-canonical-total column family exists")
    }

    /// CF for storing per account total supercharged block prodution counts
    /// ```
    /// key: [PublicKey] bytes
    /// val: [u32] BE bytes
    fn block_production_pk_supercharged_total_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-production-pk-supercharged-total")
            .expect("block-production-pk-supercharged-total column family exists")
    }

    /// CF for storing per epoch block production counts
    /// ```
    /// key: {genesis}{epoch}
    /// val: [u32] BE bytes
    /// where
    /// - genesis: [StateHash] bytes
    /// - epoch:   [u32] BE bytes
    /// ```
    /// Use [epoch_key]
    fn block_production_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-production-epoch")
            .expect("block-production-epoch column family exists")
    }

    /// CF for storing per epoch canonical block production counts
    /// ```
    /// key: {genesis}{epoch}
    /// val: [u32] BE bytes
    /// where
    /// - genesis: [StateHash] bytes
    /// - epoch:   [u32] BE bytes
    /// ```
    /// Use [epoch_key]
    fn block_production_canonical_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-production-canonical-epoch")
            .expect("block-production-canonical-epoch column family exists")
    }

    /// CF for storing per epoch supercharged block production counts
    /// ```
    /// key: {genesis}{epoch}
    /// val: [u32] BE bytes
    /// where
    /// - genesis: [StateHash] bytes
    /// - epoch:   [u32] BE bytes
    /// ```
    /// Use [epoch_key]
    fn block_production_supercharged_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-production-supercharged-epoch")
            .expect("block-production-supercharged-epoch column family exists")
    }

    /// CF for storing per block SNARK counts
    /// ```
    /// key: [StateHash] bytes
    /// val: [u32] BE bytes
    fn block_snark_counts_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-snark-counts")
            .expect("block-snark-counts column family exists")
    }

    /// CF for stoing per block user command counts
    /// ```
    /// key: [StateHash] bytes
    /// val: [u32] BE bytes
    fn block_user_command_counts_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-user-command-counts")
            .expect("block-user-command-counts column family exists")
    }

    /// CF for stoing per block zkapp command counts
    /// ```
    /// key: [StateHash] bytes
    /// val: [u32] BE bytes
    fn block_zkapp_command_counts_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-zkapp-command-counts")
            .expect("block-zkapp-command-counts column family exists")
    }

    /// CF for storing per block internal command counts
    /// ```
    /// key: [StateHash] bytes
    /// val: [u32] BE bytes
    fn block_internal_command_counts_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-internal-command-counts")
            .expect("block-internal-command-counts column family exists")
    }

    /// CF for storing per epoch slots produced
    /// ```
    /// key: {genesis}{epoch}
    /// val: [u32] BE bytes
    /// where
    /// - genesis: [StateHash] bytes
    /// - epoch:   [u32] BE bytes
    /// ```
    /// Use [epoch_key]
    fn block_epoch_slots_produced_count_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-epoch-slots-produced-count")
            .expect("block-epoch-slots-produced-count column family exists")
    }

    /// CF for storing per epoch per account slots produced
    /// ```
    /// key: {genesis}{epoch}{pk}
    /// val: [u32] BE bytes
    /// where
    /// - genesis: [StateHash] bytes
    /// - epoch:   [u32] BE bytes
    /// - pk:      [PublicKey] bytes
    /// ```
    /// Use [epoch_pk_key]
    fn block_pk_epoch_slots_produced_count_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-pk-epoch-slots-produced-count")
            .expect("block-pk-epoch-slots-produced-count column family exists")
    }

    /// CF for storing per epoch per account slots produced
    /// ```
    /// key: {genesis}{epoch}{count}{pk}
    /// val: b""
    /// where
    /// - genesis: [StateHash] bytes
    /// - epoch:   [u32] BE bytes
    /// - count:   [u32] BE bytes
    /// - pk:      [PublicKey] bytes
    /// ```
    /// Use [epoch_num_pk_key]
    fn block_pk_epoch_slots_produced_count_sort_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("block-pk-epoch-slots-produced-count-sort")
            .expect("block-pk-epoch-slots-produced-count-sort column family exists")
    }

    /// CF for storing per epoch per account user command counts
    /// ```
    /// key: {genesis}{epoch}{pk}
    /// val: [u32] BE bytes
    /// where
    /// - genesis: [StateHash] bytes
    /// - epoch:   [u32] BE bytes
    /// - pk:      [PublicKey] bytes
    /// ```
    /// Use [epoch_pk_key]
    fn user_commands_pk_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("user-commands-pk-epoch")
            .expect("user-commands-pk-epoch column family exists")
    }

    /// CF for storing per epoch per account zkapp command counts
    /// ```
    /// key: {genesis}{epoch}{pk}
    /// val: [u32] BE bytes
    /// where
    /// - genesis: [StateHash] bytes
    /// - epoch:   [u32] BE bytes
    /// - pk:      [PublicKey] bytes
    /// ```
    /// Use [epoch_pk_key]
    fn zkapp_commands_pk_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-commands-pk-epoch")
            .expect("zkapp-commands-pk-epoch column family exists")
    }

    /// CF for storing per account total user command counts
    /// ```
    /// key: [PublicKey] bytes
    /// val: [u32] BE bytes
    fn user_commands_pk_total_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("user-commands-pk-total")
            .expect("user-commands-pk-total column family exists")
    }

    /// CF for storing per account total zkapp command counts
    /// ```
    /// key: [PublicKey] bytes
    /// val: [u32] BE bytes
    fn zkapp_commands_pk_total_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-commands-pk-total")
            .expect("zkapp-commands-pk-total column family exists")
    }

    /// CF for per epoch user command counts
    /// ```
    /// key: {genesis}{epoch}
    /// val: [u32] BE bytes
    /// where
    /// - genesis: [StateHash] bytes
    /// - epoch:   [u32] BE bytes
    /// ```
    /// Use [epoch_key]
    fn user_commands_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("user-commands-epoch")
            .expect("user-commands-epoch column family exists")
    }

    /// CF for per epoch zkapp command counts
    /// ```
    /// key: {genesis}{epoch}
    /// val: [u32] BE bytes
    /// where
    /// - genesis: [StateHash] bytes
    /// - epoch:   [u32] BE bytes
    /// ```
    /// Use [epoch_key]
    fn zkapp_commands_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("zkapp-commands-epoch")
            .expect("zkapp-commands-epoch column family exists")
    }

    /// CF for storing per epoch per account internal command counts
    /// ```
    /// key: {genesis}{epoch}{pk}
    /// val: [u32] BE bytes
    /// where
    /// - genesis: [StateHash] bytes
    /// - epoch:   [u32] BE bytes
    /// - pk:      [PublicKey] bytes
    /// ```
    /// Use [epoch_pk_key]
    fn internal_commands_pk_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("internal-commands-pk-epoch")
            .expect("internal-commands-pk-epoch column family exists")
    }

    /// CF for storing per account total internal command counts
    /// ```
    /// key: [PublicKey] bytes
    /// val: [u32] BE bytes
    fn internal_commands_pk_total_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("internal-commands-pk-total")
            .expect("internal-commands-pk-total column family exists")
    }

    /// CF for storing per epoch total internal command counts
    /// ```
    /// key: {genesis}{epoch}
    /// val: [u32] BE bytes
    /// where
    /// - genesis: [StateHash] bytes
    /// - epoch:   [u32] BE bytes
    /// ```
    /// Use [epoch_key]
    fn internal_commands_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("internal-commands-epoch")
            .expect("internal-commands-epoch column family exists")
    }

    /// CF for storing per epoch per account SNARK counts
    /// ```
    /// key: {genesis}{epoch}{pk}
    /// val: [u32] BE bytes
    /// where
    /// - genesis: [StateHash] bytes
    /// - epoch:   [u32] BE bytes
    /// - pk:      [PublicKey] bytes
    /// ```
    /// Use [epoch_pk_key]
    fn snarks_pk_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snarks-pk-epoch")
            .expect("snarks-pk-epoch column family exists")
    }

    /// CF for storing per account total SNARK counts
    /// ```
    /// key: [PublicKey] bytes
    /// val: [u32] BE bytes
    fn snarks_pk_total_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snarks-pk-total")
            .expect("snarks-pk-total column family exists")
    }

    /// CF for storing per epoch SNARK counts
    /// ```
    /// key: {genesis}{epoch}
    /// val: [u32] BE bytes
    /// where
    /// - genesis: [StateHash] bytes
    /// - epoch:   [u32] BE bytes
    /// ```
    /// Use [epoch_key]
    fn snarks_epoch_cf(&self) -> &ColumnFamily {
        self.database
            .cf_handle("snarks-epoch")
            .expect("snarks-epoch column family exists")
    }
}
