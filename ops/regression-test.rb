#! /usr/bin/env -S ruby -w
# frozen_string_literal: true

# -*- mode: ruby -*-

VOLUMES_DIR = ENV['VOLUMES_DIR'] || '/mnt'
BASE_DIR = "#{VOLUMES_DIR}/mina-indexer-dev"

unless File.exist?(BASE_DIR)
  abort <<-ABORT
    #{BASE_DIR} must exist to perform regression tests.
    Failure.
  ABORT
end

test_names = %w[
  indexer_cli_reports
  server_startup
  ipc_is_available_immediately
  startup_dirs_get_created
  clean_shutdown
  clean_kill
  account_balance_cli
  account_public_key_json
  canonical_root
  canonical_threshold
  best_tip
  blocks
  block_copy
  missing_blocks
  missing_block_recovery
  best_chain
  block_children
  ledgers
  sync
  replay
  transactions
  snark_work
  snapshot
  database_create
  snapshot_database_dir
  rest_accounts_summary
  rest_blocks
  genesis_block_creator
  txn_nonces
  startup_staking_ledgers
  watch_staking_ledgers
  staking_delegations
  internal_commands
  start_from_config
  version_file
  hurl
]

# long_test_names = ['test_many_blocks', 'test_release']

puts 'Regression testing...'
BASH_TEST_DRIVER = "#{__dir__}/../tests/regression.bash"
EXE = ARGV.shift
tests = if ARGV.empty?
          # Run all tests, but not the long-running ones.
          test_names
        else
          ARGV
        end
tests.each do |tn|
  system(BASH_TEST_DRIVER, EXE, "test_#{tn}") ||
    abort("Failure from: #{BASH_TEST_DRIVER} #{EXE} test_#{tn}")
end
puts 'Regression testing complete.'
