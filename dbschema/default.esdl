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
    required height: int64 {
        constraint min_value(0);
    }
    required epoch: int64;
    required global_slot_since_genesis: int64 {
        constraint min_value(0);
    }
    required curr_global_slot_number: int64;
    required scheduled_time: int64;
    required total_currency: int64;
    required stake_winner: Account;
    required creator: Account;
    required coinbase_target: Account;
    required supercharge_coinbase: bool;
    required min_window_density: int64;
    required has_ancestor_in_same_checkpoint_window: bool;
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
  }

  type StakingEpochData extending EpochData {}
  type NextEpochData extending EpochData {}

  abstract type UserCommand {
    required block: Block {
      on target delete restrict;
    }
    required status: str;
    source: Account;
    source_balance: decimal;
    target: Account;
    target_balance: decimal;
    fee: decimal;
    fee_payer: Account;
    fee_payer_balance: decimal;
    fee_token: str;
    fee_payer_account_creation_fee_paid: decimal;
    target_account_creation_fee_paid: decimal;
    nonce: int64;
    valid_until: int64;
    memo: str;
    signer: Account;
    signature: str;
    created_token: str;
  }

  type Payment extending UserCommand {
    token_id: int64;
    amount: decimal;
  }

  type StakingDelegation extending UserCommand {}

  abstract type InternalCommand {
    required block: Block {
      on target delete restrict;
    }
  }

  type Coinbase extending InternalCommand {
    required target_balance: decimal;
  }

  type FeeTransfer extending InternalCommand {
    required target1_balance: decimal;
    target2_balance: decimal;
  }

  type StakingEpoch {
    required hash: str {
      constraint regexp(r"^j.{50}$");
    };
    required epoch: int64;
    constraint exclusive on ((.hash, .epoch));
  }

  type StakingLedger {
    required epoch: StakingEpoch;
    required source: Account;
    required balance: decimal;
    required target: Account;
    required token: int64;
    nonce: int64;
    required receipt_chain_hash: str {
      constraint regexp(r"^2.{51}$");
    };
    required voting_for: str {
      constraint regexp(r"^3N[A-Za-z].{49}$");
    };
  }

  type StakingTiming {
    required ledger: StakingLedger {
      on target delete restrict;
    }
    required initial_minimum_balance: decimal;
    required cliff_time: int64;
    required cliff_amount: decimal;
    required vesting_period: int64;
    required vesting_increment: decimal;
  }
}
