#!/usr/bin/env -S ruby -w

# -*- mode: ruby -*-

DEPLOY_TYPE = ARGV[0]       # 'test' or 'prod'
BLOCKS_COUNT = ARGV[1]      # number of blocks to deploy
WEB_PORT = ARGV[2] || 8080  # optional web port for server

VOLUMES_DIR = ENV["VOLUMES_DIR"] || "/mnt"
BASE_DIR = "#{VOLUMES_DIR}/mina-indexer-#{DEPLOY_TYPE}"

require "fileutils"
require "#{__dir__}/helpers" # Expects BASE_DIR to be defined

abort "Error: #{BASE_DIR} must exist to start the server." unless File.exist?(BASE_DIR)

overall_success = true

# Terminate the current version, if any.
if File.exist? CURRENT
  current = File.read(CURRENT)
  if current != REV
    socket = "#{BASE_DIR}/mina-indexer-#{current}.sock"
    shutdown_success = system(
      EXE,
      "--socket", socket,
      "server", "shutdown"
    )
    puts("Shutting down (via command line and socket #{socket}) failed. Moving on.") unless shutdown_success
    overall_success &&= shutdown_success

    # Give the process a second to clean up.
    sleep 1
  end
end

# Now, we take over.
File.write(CURRENT, REV)

if DEPLOY_TYPE == "test"
  puts "Restarting server..."
  PORT = random_port
  pid = spawn EXE +
    " --socket #{SOCKET} " \
    " server start" \
    " --log-level DEBUG" \
    " --web-port #{PORT}" \
    " --database-dir #{db_dir(BLOCKS_COUNT)}" \
    " >> #{LOGS_DIR}/out 2>> #{LOGS_DIR}/err"
  wait_for_socket(10)
  puts "Server restarted."

  # Create an indexer db snapshot to restore from later
  puts "Creating snapshot at #{snapshot_path(BLOCKS_COUNT)}..."
  config_snapshots_dir
  snapshot_success = system(
    EXE,
    "--socket", SOCKET,
    "database", "snapshot",
    "--output-path", snapshot_path(BLOCKS_COUNT)
  )
  puts "Snapshot creation failed." unless snapshot_success
  overall_success &&= snapshot_success

  # Restore database from the snapshot made earlier
  puts "Testing snapshot restore of #{snapshot_path(BLOCKS_COUNT)}..."
  restore_path = "#{BASE_DIR}/restore-#{REV}.tmp"
  restore_success = system(
    EXE,
    "database", "restore",
    "--snapshot-file", snapshot_path(BLOCKS_COUNT),
    "--restore-dir", restore_path
  )
  puts "Snapshot restore failed." unless restore_success
  overall_success &&= restore_success

  # Shutdown indexer after testing
  puts "Initiating shutdown..."
  shutdown_test_success = system(
    EXE,
    "--socket", SOCKET,
    "shutdown"
  )
  puts "Shutdown after test failed." unless shutdown_test_success
  overall_success &&= shutdown_test_success
  Process.wait(pid)

  # Delete the snapshot and restore directories
  FileUtils.rm_rf(restore_path)
  File.unlink(snapshot_path(BLOCKS_COUNT))

  # Delete the CURRENT revision file after testing completes
  File.delete(CURRENT)
else
  # Daemonize the EXE for production.
  pid = fork
  if pid
    Process.detach pid
    puts "Session dispatched with PID #{pid}. Parent exiting."
  else
    Process.setsid
    pid = spawn EXE +
      " --socket #{SOCKET} " \
      " server start" \
      " --log-level DEBUG" \
      " --web-hostname 0.0.0.0" \
      " --web-port #{WEB_PORT}" \
      " --database-dir #{db_dir(BLOCKS_COUNT)}" \
      " >> #{LOGS_DIR}/out 2>> #{LOGS_DIR}/err"
    Process.detach pid
    puts "Mina Indexer daemon dispatched with PID #{pid}. Web port: #{WEB_PORT}. Child exiting."
  end
end

exit overall_success ? 0 : 1
