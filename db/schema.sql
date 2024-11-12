CREATE SEQUENCE IF NOT EXISTS epoch_data_id_seq;
CREATE SEQUENCE IF NOT EXISTS user_commands_id_seq;
CREATE SEQUENCE IF NOT EXISTS internal_commands_id_seq;
CREATE SEQUENCE IF NOT EXISTS snark_jobs_id_seq;
CREATE SEQUENCE IF NOT EXISTS staking_ledgers_id_seq;
CREATE SEQUENCE IF NOT EXISTS staking_epochs_id_seq;

-- Create tables
CREATE TABLE IF NOT EXISTS accounts (
    public_key VARCHAR PRIMARY KEY CHECK (public_key SIMILAR TO 'B62[0-9A-Za-z]{52}')
);

CREATE TABLE IF NOT EXISTS blocks (
    hash VARCHAR PRIMARY KEY CHECK (hash SIMILAR TO '3N[A-Za-z][0-9A-Za-z]{49}'),
    previous_hash VARCHAR(52),
    genesis_hash VARCHAR(52),
    blockchain_length BIGINT CHECK (blockchain_length >= 0),
    epoch BIGINT,
    global_slot_since_genesis BIGINT CHECK (global_slot_since_genesis >= 0),
    scheduled_time BIGINT,
    total_currency BIGINT,
    stake_winner VARCHAR REFERENCES accounts(public_key),
    creator VARCHAR REFERENCES accounts(public_key),
    coinbase_target VARCHAR REFERENCES accounts(public_key),
    supercharge_coinbase BOOLEAN,
    last_vrf_output VARCHAR,
    min_window_density BIGINT,
    has_ancestor_in_same_checkpoint_window BOOLEAN
);

CREATE TABLE IF NOT EXISTS blockchain_states (
    block_hash VARCHAR PRIMARY KEY REFERENCES blocks(hash),
    snarked_ledger_hash VARCHAR(52),
    genesis_ledger_hash VARCHAR(52),
    snarked_next_available_token BIGINT,
    timestamp BIGINT
);

CREATE TABLE IF NOT EXISTS staged_ledger_hashes (
    block_hash VARCHAR PRIMARY KEY REFERENCES blockchain_states(block_hash),
    non_snark_ledger_hash VARCHAR,
    non_snark_aux_hash VARCHAR,
    non_snark_pending_coinbase_aux VARCHAR,
    pending_coinbase_hash VARCHAR
);

CREATE TABLE IF NOT EXISTS epoch_data (
    id BIGINT DEFAULT(nextval('epoch_data_id_seq')) PRIMARY KEY,
    block_hash VARCHAR REFERENCES blocks(hash),
    ledger_hash VARCHAR,
    total_currency BIGINT,
    seed VARCHAR,
    start_checkpoint VARCHAR,
    lock_checkpoint VARCHAR,
    epoch_length BIGINT,
    type VARCHAR CHECK (type IN ('staking', 'next'))
);

CREATE TABLE IF NOT EXISTS user_commands (
    id BIGINT DEFAULT(nextval('user_commands_id_seq')) PRIMARY KEY,
    block_hash VARCHAR REFERENCES blocks(hash),
    status VARCHAR,
    source VARCHAR REFERENCES accounts(public_key),
    source_balance DECIMAL,
    target VARCHAR REFERENCES accounts(public_key),
    target_balance DECIMAL,
    fee DECIMAL,
    fee_payer VARCHAR REFERENCES accounts(public_key),
    fee_payer_balance DECIMAL,
    fee_token VARCHAR,
    fee_payer_account_creation_fee_paid DECIMAL,
    target_account_creation_fee_paid DECIMAL,
    nonce BIGINT,
    valid_until BIGINT,
    memo VARCHAR,
    signer VARCHAR REFERENCES accounts(public_key),
    signature VARCHAR,
    created_token VARCHAR,
    type VARCHAR CHECK (type IN ('payment', 'staking_delegation')),
    token_id BIGINT,
    amount DECIMAL
);

CREATE TABLE IF NOT EXISTS internal_commands (
    id BIGINT DEFAULT(nextval('internal_commands_id_seq')) PRIMARY KEY,
    block_hash VARCHAR REFERENCES blocks(hash),
    type VARCHAR CHECK (type IN ('coinbase', 'fee_transfer')),
    target1_balance DECIMAL,
    target2_balance DECIMAL
);

CREATE TABLE IF NOT EXISTS snark_jobs (
    id BIGINT DEFAULT(nextval('snark_jobs_id_seq')) PRIMARY KEY,
    block_hash VARCHAR REFERENCES blocks(hash),
    prover VARCHAR REFERENCES accounts(public_key),
    fee DECIMAL
);

CREATE TABLE IF NOT EXISTS staking_epochs (
    id BIGINT DEFAULT(nextval('staking_epochs_id_seq')) PRIMARY KEY,
    hash VARCHAR NOT NULL CHECK (hash SIMILAR TO 'j[0-9A-Za-z]{50}'),
    epoch BIGINT NOT NULL,
    UNIQUE(hash, epoch)
);

CREATE TABLE IF NOT EXISTS staking_ledgers (
    id BIGINT DEFAULT(nextval('staking_ledgers_id_seq')) PRIMARY KEY,
    staking_epoch_id BIGINT NOT NULL REFERENCES staking_epochs(id),
    source VARCHAR REFERENCES accounts(public_key),
    balance DECIMAL,
    target VARCHAR REFERENCES accounts(public_key),
    token VARCHAR,
    nonce BIGINT,
    receipt_chain_hash VARCHAR CHECK (receipt_chain_hash SIMILAR TO '2[0-9A-Za-z]{51}'),
    voting_for VARCHAR CHECK (voting_for SIMILAR TO '3N[A-Za-z][0-9A-Za-z]{49}')
);

CREATE TABLE IF NOT EXISTS staking_timing (
    ledger_id BIGINT PRIMARY KEY REFERENCES staking_ledgers(id),
    initial_minimum_balance DECIMAL,
    cliff_time BIGINT,
    cliff_amount DECIMAL,
    vesting_period BIGINT,
    vesting_increment DECIMAL
);
