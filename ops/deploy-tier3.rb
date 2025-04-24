#!/usr/bin/env -S ruby -w

BUILD_TYPE = ARGV[0]        # 'nix' or 'dev' or 'release'
BLOCKS_COUNT = ARGV[1].to_i # number of blocks to deploy

require "#{__dir__}/ops-common"

puts "Deploying (#{DEPLOY_TYPE}) with #{BLOCKS_COUNT} blocks."

skippable = File.exist?(snapshot_path(BLOCKS_COUNT)) && BUILD_TYPE != "nix"
if !skippable

  # Configure the directories as needed.
  #

  config_base_dir
  config_exe_dir
  config_log_dir
  stage_blocks BLOCKS_COUNT
  fetch_ledgers

  ###################
  # Create database #
  ###################

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

    # The version expected to be currently running is the one given in CURRENT
    #
    current = File.read(CURRENT)

    # The socket used for that mina-indexer is named after the version.
    #
    socket = socket_from_rev(current)

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

  ################
  # Start server #
  ################

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

  #####################
  # Database snapshot #
  #####################

  puts "Creating snapshot at #{snapshot_path(BLOCKS_COUNT)}..."
  invoke_mina_indexer(
    "--socket", SOCKET,
    "database", "snapshot",
    "--output-path", snapshot_path(BLOCKS_COUNT)
  ) || abort("Snapshot creation failed. Aborting.")
  puts "Snapshot complete."

  if BUILD_TYPE == "nix"
    puts "Uploading."
    system(
      "#{SRC_TOP}/ops/upload-snapshot.sh",
      File.basename(snapshot_path(BLOCKS_COUNT)),
      {chdir: File.dirname(snapshot_path(BLOCKS_COUNT))}
    ) || abort("Snapshot upload failed. Aborting.")
  end

  #####################
  # Ledger diff tests #
  #####################

  # Compare the indexer ledger at height #{height} with the corresponding Mina
  # ledger.
  #
  def check_ledger(height)
    puts "Attempting ledger extraction at height #{height}..."

    idxr_ledger = "#{LOGS_DIR}/ledger-#{height}.json"
    unless system(
      EXE,
      "--socket", SOCKET,
      "ledgers",
      "height",
      "--height", height.to_s,
      "--path", idxr_ledger
    )
      abort("Ledger extraction failed.")
    end

    puts "Ledger extraction complete. Verifying ledger at #{height}..."

    idxr_norm_ledger = "#{idxr_ledger}.norm.json"
    unless system(
      "#{SRC_TOP}/ops/indexer-ledger-normalizer.rb",
      idxr_ledger,
      out: idxr_norm_ledger
    )
      abort("Normalizing indexer ledger at height #{height} failed.")
    end

    mina_norm_ledgers = Dir["#{SRC_TOP}/tests/data/ledgers/ledger-#{height}-*.norm.json"]
    unless mina_norm_ledgers.length == 1
      abort "There is not exactly 1 normalized ledger against which to check."
    end

    idxr_ledger_diff = "#{LOGS_DIR}/ledger-#{height}.diff"
    unless system(
      "diff --unified #{idxr_norm_ledger} #{mina_norm_ledgers[0]}",
      out: idxr_ledger_diff
    )
      abort("Regression introduced to ledger calculations. Inspect diff file: #{idxr_ledger_diff}")
    end
  end

  if BLOCKS_COUNT >= 359604
    check_ledger(359604)
  end

  if BLOCKS_COUNT >= 427023
    check_ledger(427023)
  end

  #########################
  # Restore from snapshot #
  #########################

  puts "Testing snapshot restore of #{snapshot_path(BLOCKS_COUNT)}..."
  restore_path = "#{BASE_DIR}/restore-#{BLOCKS_COUNT}.#{REV}.tmp"
  invoke_mina_indexer(
    "database", "restore",
    "--snapshot-file", snapshot_path(BLOCKS_COUNT),
    "--restore-dir", restore_path
  ) || abort("Snapshot restore failed.")
  puts "Snapshot restore complete."

  ##############
  # Shutdown 1 #
  ##############

  puts "Initiating shutdown 1..."
  invoke_mina_indexer(
    "--socket", SOCKET,
    "shutdown"
  ) || abort("Shutdown failed after snapshot.")
  Process.wait(pid)
  puts "Shutdown 1 complete."

  ##############
  # Self-check #
  ##############

  puts "Initiating self-check with the restored database..."
  command_line = EXE +
    " --socket #{SOCKET} " \
    " server start" \
    " --self-check" \
    " --log-level DEBUG" \
    " --web-port #{PORT}" \
    " --database-dir #{restore_path}" \
    " >> #{LOGS_DIR}/out 2>> #{LOGS_DIR}/err"
  pid = spawn({"RUST_BACKTRACE" => "full"}, command_line)
  wait_for_socket(10)
  puts "Self-check complete."

  ##############
  # Shutdown 2 #
  ##############

  puts "Initiating shutdown 2..."
  invoke_mina_indexer(
    "--socket", SOCKET,
    "shutdown"
  ) || abort("Shutdown failed after self-check.")
  Process.wait(pid)
  puts "Shutdown 2 complete."

  File.delete(CURRENT)

  # Delete the snapshot and the database directory restored to.
  #
  FileUtils.rm_rf(restore_path)
  if BUILD_TYPE == "nix"
    File.unlink(snapshot_path(BLOCKS_COUNT))
  end

  # Delete the database directory. We have the snapshot if we want it.
  #
  FileUtils.rm_rf(db_dir(BLOCKS_COUNT))
end
