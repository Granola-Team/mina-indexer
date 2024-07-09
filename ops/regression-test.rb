#! /usr/bin/env -S ruby -w
# frozen_string_literal: true

# -*- mode: ruby -*-

VOLUMES_DIR = ENV['VOLUMES_DIR'] || '/mnt'
DEV_DIR = "#{VOLUMES_DIR}/mina-indexer-dev"

abort "Failure: #{DEV_DIR} must exist." unless File.exist?(DEV_DIR)

require 'fileutils'

REV = `git rev-parse --short=8 HEAD`.chomp
BASE_DIR = "#{DEV_DIR}/rev-#{REV}"
FileUtils.mkdir_p BASE_DIR

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

def cleanup_socket
  # Attempt a regular shutdown if the socket is present.
  socket = "#{BASE_DIR}/mina-indexer.sock"
  return unless File.exist?(socket)

  system(
    EXE,
    '--socket', socket,
    'server', 'shutdown'
  ) || warn("Shutdown failed despite #{socket} existing.")

  sleep 1 # Give it a chance to shut down.
end

def cleanup_idxr_pid
  pid_file = "#{BASE_DIR}/idxr_pid"
  return unless File.exist?(pid_file)

  pid = File.read pid_file
  Process.kill('HUP', pid)
  File.unlink pid_file
  sleep 1 # Give it a chance to shut down.
end

def cleanup_database_pid
  pid_file = "#{BASE_DIR}/database/PID"
  return unless File.exist?(pid_file)

  pid = File.read pid_file
  Process.kill('HUP', pid)
  sleep 1 # Give it a chance to shut down.
end

def cleanup
  cleanup_socket
  cleanup_idxr_pid
  cleanup_database_pid
end

tests.each do |tn|
  puts
  puts "Testing: #{tn}"
  test_success = system(BASH_TEST_DRIVER, EXE, "test_#{tn}")
  cleanup
  test_success || abort("Failure from: #{BASH_TEST_DRIVER} #{EXE} test_#{tn}")
end

puts 'Regression testing complete.'
