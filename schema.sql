DROP TABLE IF EXISTS `command_status`;
DROP TABLE IF EXISTS `commands`;
DROP TABLE IF EXISTS `staged_ledger_hash`;
DROP TABLE IF EXISTS `blockchain_state`;
DROP TABLE IF EXISTS `epoch_data`;
DROP TABLE IF EXISTS `consensus_state`;
DROP TABLE IF EXISTS `protocol_state`;
DROP TABLE IF EXISTS `accounts`;
DROP TABLE IF EXISTS `coinbase`;
DROP TABLE IF EXISTS `epoch_data`;
DROP TABLE IF EXISTS `fee_transfer`;

CREATE TABLE
accounts (id VARCHAR(55) PRIMARY KEY);

CREATE TABLE
protocol_state (
    block_hash VARCHAR(52) PRIMARY KEY,
    previous_state_hash VARCHAR(52) NOT NULL,
    genesis_state_hash VARCHAR(52) NOT NULL,
    blockchain_length INT,
    min_window_density INT,
    total_currency BIGINT,
    global_slot_since_genesis INT,
    has_ancestor_in_same_checkpoint_window BOOLEAN,
    block_stake_winner VARCHAR(55),
    block_creator VARCHAR(55),
    coinbase_receiver VARCHAR(55),
    supercharge_coinbase BOOLEAN,
    FOREIGN KEY (block_stake_winner) REFERENCES accounts (id),
    FOREIGN KEY (block_creator) REFERENCES accounts (id),
    FOREIGN KEY (coinbase_receiver) REFERENCES accounts (id)
);

CREATE TABLE
blockchain_state (
    id VARCHAR(45) PRIMARY KEY,
    block_hash VARCHAR(52) NOT NULL,
    snarked_ledger_hash VARCHAR(52) NOT NULL,
    genesis_ledger_hash VARCHAR(52) NOT NULL,
    snarked_next_available_token INT,
    `timestamp` BIGINT,
    FOREIGN KEY (block_hash) REFERENCES protocol_state (block_hash)
);

CREATE TABLE
consensus_state (
    block_hash VARCHAR(52) PRIMARY KEY,
    epoch_count INT,
    curr_global_slot_slot_number INT,
    curr_global_slot_slots_per_epoch INT,
    FOREIGN KEY (block_hash) REFERENCES protocol_state (block_hash)
);

CREATE TABLE
staged_ledger_hash (
    id INT AUTO_INCREMENT PRIMARY KEY,
    blockchain_state_id VARCHAR(45),
    non_snark_ledger_hash VARCHAR(52),
    non_snark_aux_hash VARCHAR(52),
    non_snark_pending_coinbase_aux VARCHAR(52),
    pending_coinbase_hash VARCHAR(52),
    FOREIGN KEY (blockchain_state_id) REFERENCES blockchain_state (id)
);

CREATE TABLE
epoch_data (
    id INT AUTO_INCREMENT PRIMARY KEY,
    block_hash VARCHAR(52) NOT NULL,
    `type` ENUM('next', 'staking') NOT NULL,
    ledger_hash VARCHAR(52) NOT NULL,
    total_currency BIGINT NOT NULL,
    seed VARCHAR(52) NOT NULL,
    start_checkpoint VARCHAR(52) NOT NULL,
    lock_checkpoint VARCHAR(52) NOT NULL,
    epoch_length INT NOT NULL,
    UNIQUE (block_hash, type, ledger_hash),
    FOREIGN KEY (block_hash) REFERENCES protocol_state (block_hash)
);

CREATE TABLE
commands (
    id INT AUTO_INCREMENT PRIMARY KEY,
    fee DECIMAL(20, 2),
    fee_token VARCHAR(255),
    fee_payer_pk VARCHAR(55),
    nonce INT,
    valid_until BIGINT,
    memo VARCHAR(255),
    source_pk VARCHAR(55),
    receiver_pk VARCHAR(55),
    token_id INT,
    amount DECIMAL(20, 2),
    signer VARCHAR(55),
    signature VARCHAR(100),
    FOREIGN KEY (fee_payer_pk) REFERENCES accounts (id),
    FOREIGN KEY (source_pk) REFERENCES accounts (id),
    FOREIGN KEY (receiver_pk) REFERENCES accounts (id),
    FOREIGN KEY (signer) REFERENCES accounts (id)
);

CREATE TABLE
command_status (
    id INT AUTO_INCREMENT PRIMARY KEY,
    command_id INT,
    `status` VARCHAR(20),
    fee_payer_account_creation_fee_paid DECIMAL(20, 2),
    receiver_account_creation_fee_paid DECIMAL(20, 2),
    created_token VARCHAR(255),
    fee_payer_balance DECIMAL(20, 2),
    source_balance DECIMAL(20, 2),
    receiver_balance DECIMAL(20, 2),
    FOREIGN KEY (command_id) REFERENCES commands (id)
);

CREATE TABLE
coinbase (
    id INT AUTO_INCREMENT PRIMARY KEY,
    `type` VARCHAR(50),
    receiver_balance DECIMAL(20, 2)
);

CREATE TABLE
fee_transfer (
    id INT AUTO_INCREMENT PRIMARY KEY,
    receiver1_balance DECIMAL(20, 2),
    receiver2_balance DECIMAL(20, 2)
);
