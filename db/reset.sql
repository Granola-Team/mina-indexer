
DROP TABLE IF EXISTS staking_timing;
DROP TABLE IF EXISTS staking_ledgers;
DROP TABLE IF EXISTS staking_epochs;
DROP TABLE IF EXISTS snark_jobs;
DROP TABLE IF EXISTS internal_commands;
DROP TABLE IF EXISTS user_commands;
DROP TABLE IF EXISTS epoch_data;
DROP TABLE IF EXISTS staged_ledger_hashes;
DROP TABLE IF EXISTS blockchain_states;
DROP TABLE IF EXISTS blocks;
DROP TABLE IF EXISTS accounts;

-- Also drop the sequences
DROP SEQUENCE IF EXISTS epoch_data_id_seq;
DROP SEQUENCE IF EXISTS user_commands_id_seq;
DROP SEQUENCE IF EXISTS internal_commands_id_seq;
DROP SEQUENCE IF EXISTS snark_jobs_id_seq;
DROP SEQUENCE IF EXISTS staking_ledgers_id_seq;
