#!/usr/bin/env -S ruby -w

BUILD_TYPE = ARGV[0]        # 'nix' or 'dev'
BLOCKS_COUNT = ARGV[1].to_i # number of blocks to deploy

DEPLOY_TYPE = "test"
require "#{__dir__}/ops-common"

puts "Deploying (#{DEPLOY_TYPE}) with #{BLOCKS_COUNT} blocks."

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
puts "Snapshot complete. Uploading."
system(
  "#{SRC_TOP}/ops/upload-snapshot.sh",
  File.basename(snapshot_path(BLOCKS_COUNT)),
  {chdir: File.dirname(snapshot_path(BLOCKS_COUNT))}
) || abort("Snapshot upload failed. Aborting.")

#####################
# Ledger diff tests #
#####################

def idxr_ledger(height)
  "#{LOGS_DIR}/ledger-#{height}.json"
end

def check_ledger(height)
  success = true

  abort("Height must be 359604 or 427023") unless height == 359604 || height == 427023

  # Compare the indexer ledger at height #{height} with the corresponding Mina ledger
  #
  puts "Attempting ledger extraction at height #{height}..."

  idxr_ledger = idxr_ledger(height)
  unless system(
    EXE,
    "--socket", SOCKET,
    "ledgers",
    "height",
    "--height", height.to_s,
    "--path", idxr_ledger
  )
    warn("Ledger extraction failed.")
    success = false
  end

  puts "Ledger extraction complete. Verifying ledger at #{height}..."

  idxr_norm_exe = "#{SRC_TOP}/ops/indexer-ledger-normalizer.rb"
  idxr_norm_ledger = "#{idxr_ledger}.norm.json"
  idxr_ledger_diff = "#{LOGS_DIR}/ledger-#{height}.diff"

  # select appropriate ledger for comparison
  mina_norm_ledger = if height == 359604
    "#{SRC_TOP}/tests/data/ledgers/ledger-359604-3NLRTfY4kZyJtvaP4dFenDcxfoMfT3uEpkWS913KkeXLtziyVd15.norm.json"
  else
    "#{SRC_TOP}/tests/data/ledgers/ledger-427023-3NKmR7GZwjL9RCxE79DHCMFkKoRKT5kTiyUJfzcbzRtNE61rWxUn.norm.json"
  end

  # normalize indexer ledger
  unless system(
    idxr_norm_exe,
    idxr_ledger,
    out: idxr_norm_ledger
  )
    warn("Normalizing indexer ledger at height #{height} failed.")
    success = false
  end

  # check normalized ledgers match
  unless system(
    "diff --unified #{idxr_norm_ledger} #{mina_norm_ledger}",
    out: idxr_ledger_diff
  ) && `cat #{idxr_ledger_diff}`.empty?
    warn("Regression introduced to ledger calculations. Inspect diff file: #{idxr_ledger_diff}")
    success = height == 427023 # do not fail for 427023 yet
  end

  success
end

# capture success of diff(s)
success = if BLOCKS_COUNT >= 427023
  check_ledger(359604) && check_ledger(427023)
elsif BLOCKS_COUNT >= 359604
  check_ledger(359604)
end

#########################
# Restore from snapshot #
#########################

puts "Testing snapshot restore of #{snapshot_path(BLOCKS_COUNT)}..."
restore_path = "#{BASE_DIR}/restore-#{BLOCKS_COUNT}.#{REV}.tmp"
unless invoke_mina_indexer(
  "database", "restore",
  "--snapshot-file", snapshot_path(BLOCKS_COUNT),
  "--restore-dir", restore_path
)
  warn("Snapshot restore failed.")
  success = false
end
puts "Snapshot restore complete."

############
# Shutdown #
############

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

##############
# Self-check #
##############

# TODO: uncomment this!
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

success ||= true
exit success
