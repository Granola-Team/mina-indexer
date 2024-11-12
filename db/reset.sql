truncate staking_timing;
truncate staking_ledgers;
truncate staking_epochs;
truncate snark_jobs;
truncate internal_commands;
truncate user_commands;
truncate epoch_data;
truncate staged_ledger_hashes;
truncate blockchain_states;
truncate blocks;
truncate accounts;

-- Also drop the sequences
drop sequence if exists epoch_data_id_seq;
drop sequence if exists user_commands_id_seq;
drop sequence if exists internal_commands_id_seq;
drop sequence if exists snark_jobs_id_seq;
drop sequence if exists staking_ledgers_id_seq;
drop sequence if exists staking_epochs_id_seq;
