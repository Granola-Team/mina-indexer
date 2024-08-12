module default {
  type Account {
    required public_key: str {
      constraint exclusive;
      constraint regexp(r"^B62.{52}$");
    }
  }

  type Block {
    required hash: str {
      constraint exclusive;
      constraint regexp(r"^3N[A-Za-z].{49}$");
    }
    required scheduled_time: int64
  }

  type ProtocolState {
    required block: Block;
    required previous_state_hash: str {
        constraint max_len_value(52);
    }
    required genesis_state_hash: str {
        constraint max_len_value(52);
    }
    height: int64 {
        constraint min_value(0);
    }
    min_window_density: int64;
    total_currency: int64;
    global_slot_since_genesis: int64 {
        constraint min_value(0);
    }
    has_ancestor_in_same_checkpoint_window: bool;
    block_stake_winner: Account;
    block_creator: Account;
    coinbase_receiver: Account;
    supercharge_coinbase: bool;
  }

  type BlockchainState {
    required protocol_state: ProtocolState {
      constraint exclusive;
      on target delete restrict;
    }
    required snarked_ledger_hash: str {
        constraint max_len_value(52);
    }
    required genesis_ledger_hash: str {
        constraint max_len_value(52);
    }
    snarked_next_available_token: int64;
    timestamp: int64;
  }

  type ConsensusState {
    required protocol_state: ProtocolState {
      constraint exclusive;
      on target delete restrict;
    }
    epoch_count: int64;
    curr_global_slot_slot_number: int64;
    curr_global_slot_slots_per_epoch: int64;
  }

  type StagedLedgerHash {
    blockchain_state: BlockchainState;
    non_snark_ledger_hash: str;
    non_snark_aux_hash: str;
    non_snark_pending_coinbase_aux: str;
    pending_coinbase_hash: str;
  }

  scalar type EpochDataType extending enum<next, staking>;

  type EpochData {
    required protocol_state: ProtocolState {
      on target delete restrict;
    }
    required type: EpochDataType;
    required ledger_hash: str;
    required total_currency: int64;
    required seed: str;
    required start_checkpoint: str;
    required lock_checkpoint: str;
    required epoch_length: int64;
    constraint exclusive on ((.protocol_state, .type, .ledger_hash));
  }

  type Command {
    required block: Block;
    fee: decimal;
    fee_token: str;
    fee_payer: Account;
    nonce: int64;
    valid_until: int64;
    memo: str;
    source: Account;
    receiver: Account;
    token_id: int64;
    amount: decimal;
    signer: Account;
    signature: str;
  }

  type CommandStatus {
    command: Command;
    status: str;
    fee_payer_account_creation_fee_paid: decimal;
    receiver_account_creation_fee_paid: decimal;
    created_token: str;
    fee_payer_balance: decimal;
    source_balance: decimal;
    receiver_balance: decimal;
  }

  type Coinbase {
    required block: Block;
    required receiver_balance: decimal;
  }

  type FeeTransfer {
    required block: Block;
    receiver1_balance: decimal;
    receiver2_balance: decimal;
  }
}
