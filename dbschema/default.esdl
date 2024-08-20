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
    required previous_hash: str {
        constraint max_len_value(52);
    }
    required genesis_hash: str {
        constraint max_len_value(52);
    }
    height: int64 {
        constraint min_value(0);
    }
    global_slot_since_genesis: int64 {
        constraint min_value(0);
    }
    required scheduled_time: int64;
    total_currency: int64;
    stake_winner: Account;
    creator: Account;
    coinbase_receiver: Account;
    supercharge_coinbase: bool;
    min_window_density: int64;
    has_ancestor_in_same_checkpoint_window: bool;
  }

  type BlockchainState {
    required block: Block {
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
    required block: Block {
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

  abstract type EpochData {
    required block: Block {
      on target delete restrict;
    }
    required ledger_hash: str;
    required total_currency: int64;
    required seed: str;
    required start_checkpoint: str;
    required lock_checkpoint: str;
    required epoch_length: int64;
    constraint exclusive on ((.block, .ledger_hash));
  }

  type StakingEpochData extending EpochData {}
  type NextEpochData extending EpochData {}

  abstract type Command {
    required block: Block {
      on target delete restrict;
    }
    required status: str;
    source: Account;
    source_balance: decimal;
    receiver: Account;
    receiver_balance: decimal;
    fee: decimal;
    fee_payer: Account;
    fee_payer_balance: decimal;
    fee_token: str;
    fee_payer_account_creation_fee_paid: decimal;
    receiver_account_creation_fee_paid: decimal;
    nonce: int64;
    valid_until: int64;
    memo: str;
    signer: Account;
    signature: str;
    created_token: str;
  }

  type Payment extending Command {
    token_id: int64;
    amount: decimal;
  }

  type StakingDelegation extending Command {}

  type Coinbase {
    required block: Block {
      on target delete restrict;
    }
    required receiver_balance: decimal;
  }

  type FeeTransfer {
    required block: Block {
      on target delete restrict;
    }
    receiver1_balance: decimal;
    receiver2_balance: decimal;
  }
}
