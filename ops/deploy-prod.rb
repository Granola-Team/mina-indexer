#!/usr/bin/env -S ruby -w

BUILD_TYPE = ARGV[0]        # 'nix' or 'dev' or 'release'
BLOCKS_COUNT = ARGV[1].to_i # number of blocks to deploy
WEB_PORT = ARGV[2] || 8080  # optional web port for server

DEPLOY_TYPE = "prod"
require "#{__dir__}/ops-common"

puts "Deploying (#{DEPLOY_TYPE}) with #{BLOCKS_COUNT} blocks."

# Configure the directories as needed.
#
config_base_dir
config_exe_dir
config_log_dir

snapshot_name = snapshot_path(BLOCKS_COUNT)
snapshot_basename = File.basename(snapshot_name)
if BUILD_TYPE == "nix"
  puts "Fetching snapshot."
  system(
    "#{SRC_TOP}/ops/download-snapshot.sh",
    snapshot_basename,
    BASE_DIR
  ) || abort("Failed to download snapshot.")
else
  puts "Copying snapshot."
  system("cp #{base_dir("test")}/#{snapshot_basename} #{snapshot_name}") || abort("Failed to move snapshot file.")
end

my_db_dir = db_dir(BLOCKS_COUNT)
restore_dir = my_db_dir + ".restore"

# The restore directory must not already exist.
#
FileUtils.rm_rf(restore_dir)

puts "Restoring the snapshot to the database directory (#{my_db_dir})."

invoke_mina_indexer(
  "database", "restore",
  "--snapshot-file", snapshot_name,
  "--restore-dir", restore_dir
) || abort("Snapshot restore failed. Aborting.")

# The snapshot restore succeeded, so we may delete the snapshot.
#
FileUtils.rm_rf(snapshot_name)

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

# Make the restored snapshot be the database directory.
#
FileUtils.rm_rf(my_db_dir)
FileUtils.mv(restore_dir, my_db_dir)

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
    " --database-dir #{my_db_dir}" \
    " --staking-ledgers-dir #{LEDGERS_DIR}" \
    " --missing-block-recovery-exe #{SRC_TOP}/ops/o1labs/block-recovery.sh" \
    " --missing-block-recovery-delay 11" \
    " --fetch-new-blocks-exe #{SRC_TOP}/ops/o1labs/block-recovery.sh" \
    " --fetch-new-blocks-delay 7" \
    " --blocks-dir #{BASE_DIR}/blocks" \
    " >> #{LOGS_DIR}/out 2>> #{LOGS_DIR}/err"
  puts "Command line: #{command_line}"
  pid = spawn({"RUST_BACKTRACE" => "full"}, command_line)
  Process.detach pid
  puts "Mina Indexer daemon dispatched with PID #{pid}. Web port: #{WEB_PORT}. Child exiting."
end
