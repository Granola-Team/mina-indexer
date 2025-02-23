#!/usr/bin/env -S ruby -w

BUILD_TYPE = ARGV[0]        # 'nix' or 'dev'
BLOCKS_COUNT = ARGV[1]      # number of blocks to deploy
WEB_PORT = ARGV[2] || 8080  # optional web port for server

DEPLOY_TYPE = "prod"
require "#{__dir__}/ops-common"

puts "Deploying (#{DEPLOY_TYPE}) with #{BLOCKS_COUNT} blocks."

success = true

# Configure the directories as needed.
#
config_base_dir
config_exe_dir
config_log_dir
stage_blocks BLOCKS_COUNT
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

exit success
