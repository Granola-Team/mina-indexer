#! /usr/bin/env -S ruby -w
# frozen_string_literal: true

# -*- mode: ruby -*-

VOLUMES_DIR = ENV['VOLUMES_DIR'] || '/mnt'
DEV_DIR = "#{VOLUMES_DIR}/mina-indexer-dev"
BUILD_TYPE = 'debug'

require 'fileutils'

# clean up dev directory
if ARGV.length == 1 && ARGV[0] == 'clean-dev'
  puts "Removing #{DEV_DIR}/rev-*"
  FileUtils.rm_rf Dir.glob("#{DEV_DIR}/rev-*")
  return
end

abort "Failure: #{DEV_DIR} must exist." unless File.exist?(DEV_DIR)

rev = `git rev-parse --short=8 HEAD`.chomp
BASE_DIR = "#{DEV_DIR}/rev-#{rev}"
FileUtils.mkdir_p BASE_DIR

require "#{__dir__}/helpers" # Expects BASE_DIR & BUILD_TYPE to exist.

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
  fetch_new_blocks
  best_chain
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
  start_from_config
  version_file
  hurl
]

puts 'Regression testing...'
BASH_TEST_DRIVER = "#{__dir__}/../tests/regression.bash"
IDXR_EXE = ARGV.shift
tests = if ARGV.empty?
          # Run all tests, but not the long-running ones.
          test_names
        elsif ARGV.length == 2 && ARGV.first == 'continue'
          # Run the supplied test and remaining
          test_names.drop_while { |test| test != ARGV.last }
        else
          ARGV
        end

def cleanup_idxr_pid
  pid_file = "#{BASE_DIR}/idxr_pid"
  return unless File.exist?(pid_file)

  pid = File.read pid_file
  begin
    Process.kill('HUP', pid.to_i)
  rescue StandardError
    nil
  end
  File.unlink pid_file
  sleep 1 # Give it a chance to shut down.
end

def cleanup_database_pid
  pid_file = "#{BASE_DIR}/database/PID"
  return unless File.exist?(pid_file)

  pid = File.read pid_file
  begin
    Process.kill('HUP', pid.to_i)
  rescue StandardError
    nil
  end
  sleep 1 # Give it a chance to shut down.
end

def remove_dirs
  %w[epoch_42_ledger.json epoch_0_staking_delegations.json epoch_0_ledger.json
     mina-indexer.sock].each do |f|
    target = "#{BASE_DIR}/#{f}"
    FileUtils.rm_f target
  end
  %w[blocks staking-ledgers database].each do |f|
    target = "#{BASE_DIR}/#{f}"
    FileUtils.rm_rf target
  end
end

def cleanup
  idxr_shutdown_via_socket(IDXR_EXE, "#{BASE_DIR}/mina-indexer.sock")
  cleanup_idxr_pid
  cleanup_database_pid
  remove_dirs
end

tests.each do |tn|
  puts "\nTesting: #{tn}"
  test_success = system(BASH_TEST_DRIVER, IDXR_EXE, "test_#{tn}")
  cleanup
  test_success || abort("Failure from: #{BASH_TEST_DRIVER} #{IDXR_EXE} test_#{tn}")
end

puts 'Regression testing complete.'
