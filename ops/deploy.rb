#!/usr/bin/env -S ruby -w

# -*- mode: ruby -*-

DEPLOY_TYPE = ARGV[0]       # 'test' or 'prod'
BUILD_TYPE = ARGV[1]        # 'nix' or 'debug'
BLOCKS_COUNT = ARGV[2]      # number of blocks to deploy
WEB_PORT = ARGV[3] || 8080  # optional web port for server

require "fileutils"
require "#{__dir__}/ops-common"

abort "Error: #{BASE_DIR} must exist to perform the deployment." unless File.exist?(BASE_DIR)

puts "Deploying (#{DEPLOY_TYPE}) with #{BLOCKS_COUNT} blocks."

success = true

# Configure the directories as needed.
#
config_exe_dir
config_log_dir
get_blocks BLOCKS_COUNT
fetch_ledgers

puts "Creating database..."
invoke_mina_indexer(
  "database", "create",
  "--log-level", "DEBUG",
  "--ledger-cadence", "5000",
  "--database-dir", db_dir(BLOCKS_COUNT),
  "--blocks-dir", blocks_dir(BLOCKS_COUNT),
  "--staking-ledgers-dir", LEDGERS_DIR,  # Comment out this line to skip staking ledger ingestion.
  "--do-not-ingest-orphan-blocks"        # Comment out this line to ingest orphan blocks.
) || abort("database creation failed")
puts "Database creation succeeded."

# Terminate the current version, if any.
#
if File.exist? CURRENT

  # The version expected to be currently running is the one given in the fille
  # CURRENT.
  #
  current = File.read(CURRENT)

  # The socket used for that mina-indexer is named after the version.
  #
  socket = "#{BASE_DIR}/mina-indexer-#{current}.sock"

  # Send the currently running Indexer the shutdown command.
  #
  invoke_mina_indexer(
    "--socket", socket,
    "server", "shutdown"
  ) || puts("Shutting down (via command line and socket #{socket}) failed. Moving on.")

  # Maybe the shutdown worked, maybe it didn't. Either way, give the process a
  # second to clean up.
  sleep 1
end

# Now, we take over.
#
File.write(CURRENT, REV)

if DEPLOY_TYPE == "test"
  puts "Restarting server..."
  PORT = random_port
  command_line = EXE +
    " --socket #{SOCKET} " \
    " server start" \
    " --log-level DEBUG" \
    " --web-port #{PORT}" \
    " --database-dir #{db_dir(BLOCKS_COUNT)}" \
    " >> #{LOGS_DIR}/out 2>> #{LOGS_DIR}/err"
  pid = spawn({"RUST_BACKTRACE" => "full"}, command_line)
  wait_for_socket(10)
  puts "Server restarted."

  # Create an indexer db snapshot to restore from later
  #
  puts "Creating snapshot at #{snapshot_path(BLOCKS_COUNT)}..."
  config_snapshots_dir
  invoke_mina_indexer(
    "--socket", SOCKET,
    "database", "snapshot",
    "--output-path", snapshot_path(BLOCKS_COUNT)
  ) || abort("Snapshot creation failed. Aborting.")
  puts "Snapshot complete."

  # TODO include ledger diff test after we get a more recent ledger
  # https://github.com/Granola-Team/mina-indexer/issues/1735

  # IDXR_LEDGER = "#{LOGS_DIR}/ledger-#{BLOCKS_COUNT}-#{REV}.json"

  # # Compare the indexer best ledger with the Mina pre-hardfork ledger
  # #
  # puts "Attempting ledger extraction..."
  # unless system(
  #   EXE,
  #   "--socket", SOCKET,
  #   "ledgers",
  #   "height",
  #   "--height", BLOCKS_COUNT.to_s,
  #   "--path", IDXR_LEDGER
  # )
  #   warn("Ledger extraction failed.")
  #   success = false
  # end
  # puts "Ledger extraction complete."

  # puts "Verifying ledger at height #{BLOCKS_COUNT} is identical to the mainnet state dump"
  # IDXR_NORM_EXE = "#{SRC_TOP}/ops/indexer-ledger-normalizer.rb"
  # IDXR_NORM_LEDGER = "#{IDXR_LEDGER}.norm.json"
  # MINA_NORM_LEDGER = "#{SRC_TOP}/tests/data/ledger-359604/mina_ledger.json"
  # IDXR_LEDGER_DIFF = "#{LOGS_DIR}/ledger-#{BLOCKS_COUNT}.diff"

  # # normalize indexer best ledger
  # unless system(
  #   IDXR_NORM_EXE,
  #   IDXR_LEDGER,
  #   out: IDXR_NORM_LEDGER
  # )
  #   warn("Normalizing Indexer Ledger at height #{BLOCKS_COUNT} failed.")
  #   success = false
  # end

  # # check ledgers match
  # unless system(
  #   "diff --unified #{IDXR_NORM_LEDGER} #{MINA_NORM_LEDGER}",
  #   out: IDXR_LEDGER_DIFF
  # ) && `cat #{IDXR_LEDGER_DIFF}`.empty?
  #   warn("Regression introduced to ledger calculations. Inspect diff file: #{IDXR_LEDGER_DIFF}")
  #   success = false
  # end

  # Restore database from the snapshot made earlier
  #
  puts "Testing snapshot restore of #{snapshot_path(BLOCKS_COUNT)}..."
  restore_path = "#{BASE_DIR}/restore-#{REV}.tmp"
  unless invoke_mina_indexer(
    "database", "restore",
    "--snapshot-file", snapshot_path(BLOCKS_COUNT),
    "--restore-dir", restore_path
  )
    warn("Snapshot restore failed.")
    success = false
  end
  puts "Snapshot restore complete."

  # Shutdown indexer
  #
  puts "Initiating shutdown..."
  unless invoke_mina_indexer(
    "--socket", SOCKET,
    "shutdown"
  )
    warn("Shutdown failed after snapshot.")
    success = false
  end
  Process.wait(pid)
  puts "Shutdown complete."
  File.delete(CURRENT)

  # Delete the snapshot and the database directory restored to.
  #
  FileUtils.rm_rf(restore_path)
  File.unlink(snapshot_path(BLOCKS_COUNT))

  # Delete the database directory. We have the snapshot if we want it.
  #
  FileUtils.rm_rf(db_dir(BLOCKS_COUNT))

  # Do a database self-check
  #
  # puts 'Initiating self-check...'
  # pid = spawn EXE +
  #             " --socket #{SOCKET}" \
  #             ' server start' \
  #             ' --self-check' \
  #             ' --log-level DEBUG' \
  #             " --web-port #{PORT}" \
  #             " --database-dir #{db_dir(BLOCKS_COUNT)}" \
  #             " >> #{LOGS_DIR}/out 2>> #{LOGS_DIR}/err"
  # wait_for_socket(10)
  # puts 'Self-check complete.'
else
  # Daemonize the EXE.
  #
  pid = fork
  if pid
    # Then I am the parent. Register disinterest in the child PID.
    Process.detach pid
    puts "Session dispatched with PID #{pid}. Parent exiting."
  else
    # I am the child. (The child gets a nil return value.)
    Process.setsid
    command_line = EXE +
      " --socket #{SOCKET} " \
      " server start" \
      " --log-level DEBUG" \
      " --web-hostname 0.0.0.0" \
      " --web-port #{WEB_PORT}" \
      " --database-dir #{db_dir(BLOCKS_COUNT)}" \
      " >> #{LOGS_DIR}/out 2>> #{LOGS_DIR}/err"
    puts "Command line: #{command_line}"
    pid = spawn({"RUST_BACKTRACE" => "full"}, command_line)
    Process.detach pid
    puts "Mina Indexer daemon dispatched with PID #{pid}. Web port: #{WEB_PORT}. Child exiting."
  end
end

exit success
