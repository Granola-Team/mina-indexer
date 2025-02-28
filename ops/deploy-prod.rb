#!/usr/bin/env -S ruby -w

BUILD_TYPE = ARGV[0]        # 'nix' or 'dev'
BLOCKS_COUNT = ARGV[1]      # number of blocks to deploy
WEB_PORT = ARGV[2] || 8080  # optional web port for server

DEPLOY_TYPE = "prod"
require "#{__dir__}/ops-common"

puts "Deploying (#{DEPLOY_TYPE}) with #{BLOCKS_COUNT} blocks."

# Configure the directories as needed.
#
config_base_dir
config_exe_dir
config_log_dir

puts "Fetching snapshot."
snapshot_name = File.basename(snapshot_path(BLOCKS_COUNT))
system(
  "#{SRC_TOP}/ops/download-snapshot.sh",
  snapshot_name,
  BASE_DIR
) || abort("Failed to download snapshot.")

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

puts "Restoring the snapshot to the database directory (#{db_dir(BLOCKS_COUNT)})."

# The restore directory must not already exist.
FileUtils.rm_rf(db_dir(BLOCKS_COUNT))

invoke_mina_indexer(
  "database", "restore",
  "--snapshot-file", "#{BASE_DIR}/#{snapshot_name}",
  "--restore-dir", db_dir(BLOCKS_COUNT)
) || abort("Snapshot restore failed. Aborting.")

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
    " --staking-ledgers-dir #{LEDGERS_DIR}" \
    " >> #{LOGS_DIR}/out 2>> #{LOGS_DIR}/err"
  puts "Command line: #{command_line}"
  pid = spawn({"RUST_BACKTRACE" => "full"}, command_line)
  Process.detach pid
  puts "Mina Indexer daemon dispatched with PID #{pid}. Web port: #{WEB_PORT}. Child exiting."
end
