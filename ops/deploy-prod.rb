#!/usr/bin/env -S ruby -w

BUILD_TYPE = ARGV[0]        # 'nix' or 'dev' or 'release'
BLOCKS_COUNT = ARGV[1].to_i # number of blocks to deploy

DEPLOY_TYPE = "prod"
require "#{__dir__}/ops-common"
require "#{__dir__}/granola-rclone.rb"

puts "Deploying (#{DEPLOY_TYPE}) with #{BLOCKS_COUNT} blocks."

# Configure the directories as needed.
#
config_base_dir
config_exe_dir
config_log_dir

snapshot_name = snapshot_path(BLOCKS_COUNT)
snapshot_basename = File.basename(snapshot_name)
if BUILD_TYPE == "nix"
  puts "Fetching snapshot..."
  granola_rclone(
    "copyto",
    "linode-granola:granola-mina-indexer-snapshots/mina-indexer-snapshots/#{snapshot_basename}",
    snapshot_name
  ) || abort("Failed to download snapshot.")
  if File.exist? snapshot_name
    puts "Snapshot fetched."
  else
    abort("No snapshot downloaded. Aborting.")
  end
else
  puts "Copying snapshot..."
  system("cp #{base_dir("test")}/#{snapshot_basename} #{snapshot_name}") || abort("Failed to move snapshot file.")
  puts "Snapshot copied."
end

my_db_dir = db_dir(BLOCKS_COUNT)
restore_dir = my_db_dir + ".restore"

# The restore directory must not already exist.
#
FileUtils.rm_rf(restore_dir)

puts "Restoring the snapshot to the database directory (#{my_db_dir})..."
invoke_mina_indexer(
  "database", "restore",
  "--snapshot-file", snapshot_name,
  "--restore-dir", restore_dir
) || abort("Snapshot restore failed. Aborting.")
puts "Restore complete."

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

  # Maybe the shutdown worked, maybe it didn't. Either way:
  #
  puts "Give the process 1 second to clean up."
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
if pid.nil?
  # I am the child. (The child gets a nil return value.)
  Process.setsid
  blocks_dir = "#{DEPLOY_DIR}/new-blocks"
  command_line = EXE +
    " --socket #{SOCKET} " \
    " server start" \
    " --log-level DEBUG" \
    " --ledger-cadence 1000" \
    " --database-dir #{my_db_dir}" \
    " --staking-ledgers-dir #{LEDGERS_DIR}" \
    " --missing-block-recovery-exe #{SRC_TOP}/ops/recover-block" \
    " --missing-block-recovery-delay 11" \
    " --fetch-new-blocks-exe #{SRC_TOP}/ops/recover-block" \
    " --fetch-new-blocks-delay 7" \
    " --blocks-dir #{blocks_dir}" \
    " >> #{LOGS_DIR}/out 2>> #{LOGS_DIR}/err"
  puts "Spawning: #{command_line}"
  pid = spawn({"RUST_BACKTRACE" => "full"}, command_line)
  puts "Mina Indexer daemon spawned with PID #{pid}. Child exiting."
end
Process.detach pid
