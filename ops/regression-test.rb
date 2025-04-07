#!/usr/bin/env -S ruby -w

BUILD_TYPE = ARGV.shift

require "#{__dir__}/ops-common"

test_names = %w[
  indexer_cli_reports
  server_startup_v1
  server_startup_v2
  ipc_is_available_immediately
  startup_dirs_get_created
  clean_shutdown
  clean_kill
  account_balance_cli
  account_public_key_json
  canonical_root
  canonical_threshold
  best_tip_v1
  best_tip_v2
  blocks
  block_copy
  missing_blocks
  missing_block_recovery
  fetch_new_blocks
  best_chain_v1
  best_chain_v2
  block_children
  ledgers
  sync
  replay
  transactions
  transactions_csv
  snark_work
  snapshot
  database_create
  reuse_databases
  snapshot_database_dir
  rest_accounts_summary
  rest_blocks
  genesis_block_creator
  txn_nonces
  startup_staking_ledgers
  watch_staking_ledgers
  do_not_ingest_orphan_blocks
  staking_delegations
  internal_commands
  internal_commands_csv
  version_file
  hurl_v1
  hurl_v2
]

puts "Regression testing..."

tests = if ARGV.empty?
  # Run all tests, but not the long-running ones.
  test_names
elsif ARGV.length == 2 && ARGV.first == "continue"
  # Run the supplied test and remaining
  test_names.drop_while { |test| test != ARGV.last }
else
  ARGV
end

def cleanup_idxr_pid
  pid_file = "#{BASE_DIR}/idxr_pid"
  return unless File.exist?(pid_file)

  pid = File.read(pid_file)
  begin
    Process.kill("HUP", pid.to_i)
  rescue
    nil
  end
  File.unlink(pid_file)
  sleep 1 # Give it a chance to shut down.
end

def cleanup_database_pid
  pid_file = "#{BASE_DIR}/database/PID"
  return unless File.exist?(pid_file)

  pid = File.read(pid_file)
  begin
    Process.kill("HUP", pid.to_i)
  rescue
    nil
  end
  sleep 1 # Give it a chance to shut down.
end

def cleanup
  idxr_shutdown_via_socket(ENV["EXE_SRC"], "#{BASE_DIR}/mina-indexer.sock")
  cleanup_idxr_pid
  cleanup_database_pid
  FileUtils.rm_rf(BASE_DIR)
end

# Run the tests
#
BASH_TEST_DRIVER = "#{__dir__}/../tests/regression.bash"

tests.each do |tn|
  puts "\nTesting: #{tn}"
  system(BASH_TEST_DRIVER, ENV["EXE_SRC"], "test_#{tn}") || abort("Failure from: #{tn}")
  cleanup
end

puts "Regression testing complete."
