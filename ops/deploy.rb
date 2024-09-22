#! /usr/bin/env -S ruby -w

DEPLOY_TYPE = ARGV[0]                   # 'test' or 'prod'
BUILD_TYPE = ARGV[1]                    # 'nix' or 'debug'
BLOCKS_COUNT = ARGV[2]                  # number of blocks to deploy
WEB_PORT = ARGV[3] || 8080              # optional web port for server
EXCLUDE_NON_CANONICAL = ARGV[4] == "y"  # optional non-canonical block exclusion (default is false)

VOLUMES_DIR = ENV["VOLUMES_DIR"] || "/mnt"
BASE_DIR = "#{VOLUMES_DIR}/mina-indexer-#{DEPLOY_TYPE}"

require "fileutils"
require "#{__dir__}/helpers" # Expects BASE_DIR to be defined

abort "Error: #{BASE_DIR} must exist to perform the deployment." unless File.exist?(BASE_DIR)

puts "Deploying (#{DEPLOY_TYPE}) with #{BLOCKS_COUNT} blocks."

success = true

# Configure the directories as needed.
#
config_base_dir
config_exe_dir
config_log_dir
get_blocks BLOCKS_COUNT
fetch_ledgers

# Create the database, if needed.
#
unless File.exist?(db_dir(BLOCKS_COUNT))
  puts "Creating database..."

  if BUILD_TYPE == "debug"
    puts "Ingest staking ledgers? (y/n)"
    ingest_staking_ledgers = $stdin.gets[0].downcase
    until ["n", "y"].include? ingest_staking_ledgers
      warn("Invalid response")
      puts "Ingest staking ledgers? (y/n)"
      ingest_staking_ledgers = $stdin.gets[0].downcase
    end

    puts "Ingest orphan blocks? (y/n)"
    ingest_orphan_blocks = $stdin.gets[0].downcase
    until ["n", "y"].include? ingest_orphan_blocks
      warn("Invalid response")
      puts "Ingest staking ledgers? (y/n)"
      ingest_orphan_blocks = $stdin.gets[0].downcase
    end

    ingest_staking_ledgers = ingest_staking_ledgers == "y"
    ingest_orphan_blocks = ingest_orphan_blocks == "y"

    if !ingest_staking_ledgers && !ingest_orphan_blocks
      system(
        EXE,
        "database", "create",
        "--log-level", "DEBUG",
        "--ledger-cadence", "5000",
        "--database-dir", db_dir(BLOCKS_COUNT),
        "--blocks-dir", blocks_dir(BLOCKS_COUNT),
        "--do-not-ingest-orphan-blocks"
      )
    elsif !ingest_staking_ledgers && ingest_orphan_blocks
      system(
        EXE,
        "database", "create",
        "--log-level", "DEBUG",
        "--ledger-cadence", "5000",
        "--database-dir", db_dir(BLOCKS_COUNT),
        "--blocks-dir", blocks_dir(BLOCKS_COUNT)
      )
    elsif ingest_staking_ledgers && !ingest_orphan_blocks
      system(
        EXE,
        "database", "create",
        "--log-level", "DEBUG",
        "--ledger-cadence", "5000",
        "--database-dir", db_dir(BLOCKS_COUNT),
        "--blocks-dir", blocks_dir(BLOCKS_COUNT),
        "--staking-ledgers-dir", LEDGERS_DIR,
        "--do-not-ingest-orphan-blocks"
      )
    else
      system(
        EXE,
        "database", "create",
        "--log-level", "DEBUG",
        "--ledger-cadence", "5000",
        "--database-dir", db_dir(BLOCKS_COUNT),
        "--blocks-dir", blocks_dir(BLOCKS_COUNT),
        "--staking-ledgers-dir", LEDGERS_DIR
      )
    end
  else
    exclude_non_canonical_blocks_flag = ""
    if EXCLUDE_NON_CANONICAL == true
      exclude_non_canonical_blocks_flag = "--do-not-ingest-orphan-blocks"
    end
    system(
      EXE,
      "database", "create",
      "--log-level", "DEBUG",
      "--ledger-cadence", "5000",
      "--database-dir", db_dir(BLOCKS_COUNT),
      "--blocks-dir", blocks_dir(BLOCKS_COUNT),
      "--staking-ledgers-dir", LEDGERS_DIR,
      exclude_non_canonical_blocks_flag
    )
  end || abort("database creation failed")
  puts "Database creation succeeded."
end

# Terminate the current version, if any.
#
if File.exist? CURRENT
  current = File.read(CURRENT)
  if current != REV
    socket = "#{BASE_DIR}/mina-indexer-#{current}.sock"
    system(
      EXE,
      "--socket", socket,
      "server", "shutdown"
    ) || puts("Shutting down (via command line and socket #{socket}) failed. Moving on.")

    # Maybe the shutdown worked, maybe it didn't. Either way, give the process
    # a second to clean up.
    sleep 1
  end
end

# Now, we take over.
#
File.write CURRENT, REV

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
  #
  puts "Creating snapshot at #{snapshot_path(BLOCKS_COUNT)}..."
  config_snapshots_dir
  system(
    EXE,
    "--socket", SOCKET,
    "database", "snapshot",
    "--output-path", snapshot_path(BLOCKS_COUNT)
  ) || abort("Snapshot creation failed. Aborting.")
  puts "Snapshot complete."

  # Compare the indexer best ledger with the Mina pre-hardfork ledger
  #
  puts "Attempting ledger extraction..."
  unless system(
    EXE,
    "--socket", SOCKET,
    "ledgers",
    "height",
    "--height", BLOCKS_COUNT.to_s,
    "--path", "#{LOGS_DIR}/ledger-#{BLOCKS_COUNT}-#{REV}.json"
  )
    warn("Ledger extraction failed.")
    success = false
  end
  puts "Ledger extraction complete."

  puts "Verifying ledger at height #{BLOCKS_COUNT} is identical to the mainnet state dump"
  IDXR_NORM_EXE = "#{SRC_TOP}/ops/indexer-ledger-normalizer.rb"
  IDXR_NORM_LEDGER = "#{LOGS_DIR}/ledger-#{BLOCKS_COUNT}-norm-#{REV}.json"
  MINA_NORM_LEDGER = "#{SRC_TOP}/tests/data/ledger-359604/mina_ledger.json"
  IDXR_LEDGER_DIFF = "#{LOGS_DIR}/ledger-#{BLOCKS_COUNT}.diff"

  # normalize indexer best ledger
  unless system(
    IDXR_NORM_EXE,
    "#{LOGS_DIR}/ledger-#{BLOCKS_COUNT}-#{REV}.json",
    out: IDXR_NORM_LEDGER
  )
    warn("Normalizing Indexer Ledger at height #{BLOCKS_COUNT} failed.")
    success = false
  end

  # check ledgers match
  unless system(
    "diff --unified #{IDXR_NORM_LEDGER} #{MINA_NORM_LEDGER}",
    out: IDXR_LEDGER_DIFF
  ) && `cat #{IDXR_LEDGER_DIFF}`.empty?
    warn("Regression introduced to ledger calculations. Inspect diff file: #{IDXR_LEDGER_DIFF}")
    success = false
  end

  # Restore database from the snapshot made earlier
  #
  puts "Testing snapshot restore of #{snapshot_path(BLOCKS_COUNT)}..."
  restore_path = "#{BASE_DIR}/restore-#{REV}.tmp"
  unless system(
    EXE,
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
  unless system(
    EXE,
    "--socket", SOCKET,
    "shutdown"
  )
    warn("Shutdown failed after snapshot.")
    success = false
  end
  Process.wait(pid)

  # Delete the snapshot and the database directory restored to.
  #
  FileUtils.rm_rf(restore_path)
  File.unlink(snapshot_path(BLOCKS_COUNT))

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

  # puts 'Initiating shutdown...'
  # system(
  #   EXE,
  #   '--socket', SOCKET,
  #   'shutdown'
  # ) || puts('Shutdown failed after snapshot.')
  # Process.wait(pid)
  # puts 'Shutdown complete.'

  File.delete(CURRENT)
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

exit success
