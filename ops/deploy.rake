#!/usr/bin/env -S ruby -w

SRC_TOP = File.dirname(__dir__)

require_relative "granola-rclone"

# These 4 env vars control the behaviour of this deployment.
#
DEPLOY_DIR = ENV["MI_DEPLOY_DIR"] || "/mnt/mina-indexer-prod"
BLOCKS_COUNT = ENV["MI_BLOCKS_COUNT"] || "5000"
EXE_SRC = ENV["MI_EXE_SRC"] || "#{SRC_TOP}/result/bin/mina-indexer"
REV = ENV["GIT_COMMIT_HASH"] || begin
  git_hash = `git -C #{SRC_TOP} rev-parse --short=8 HEAD 2>/dev/null`.strip
  git_hash.empty? ? abort("Could not determine the Git hash. Aborting.") : git_hash
end

LOG_DIR = File.join(DEPLOY_DIR, "logs")
DB_DIR = File.join(DEPLOY_DIR, "db-#{BLOCKS_COUNT}")
RESTORE_DIR = DB_DIR + ".restore"
EXE_DIR = File.join(DEPLOY_DIR, "bin")
EXE = File.join(EXE_DIR, "mina-indexer-#{REV}")
PID_FILE = File.join(DB_DIR, "PID")
SNAPSHOT_NAME = File.join(DEPLOY_DIR, "mina-#{BLOCKS_COUNT}.#{REV}.snapshot")

def invoke_mina_indexer(*args) # standard:disable Style/ArgumentsForwarding
  command_line = args.unshift(EXE).join(" ")
  puts("Executing: #{command_line}")
  system({"RUST_BACKTRACE" => "full"}, command_line) # standard:disable Style/ArgumentsForwarding
end

def in_deploy_dir
  FileUtils.mkdir_p(DEPLOY_DIR)
  Dir.chdir(DEPLOY_DIR)
end

desc "Build the #{EXE_DIR}."
file EXE_DIR do |t|
  puts "Creating executable directory: #{t.name}"
  Dir.mkdir(t.name)
end

desc "Place #{EXE}."
file EXE => [EXE_DIR] do |t|
  FileUtils.cp(EXE_SRC, t.name)
end

desc "Build the #{LOG_DIR}."
file LOG_DIR do |t|
  puts "Creating logging directory: #{t.name}"
  Dir.mkdir(t.name)
end

desc "Fetch the snapshot."
file SNAPSHOT_NAME do |t|
  in_deploy_dir
  snapshot_basename = File.basename(t.name)
  puts "Fetching snapshot..."
  granola_rclone(
    "copyto",
    "linode-granola:granola-mina-indexer-snapshots/mina-indexer-snapshots/#{snapshot_basename}",
    t.name
  ) || abort("Failed to download snapshot.")
  if File.exist? t.name
    puts "Snapshot fetched."
  else
    abort("No snapshot downloaded. Aborting.")
  end
  FileUtils.touch(t.name)
end

desc "Restore the snapshot."
file RESTORE_DIR => [EXE, SNAPSHOT_NAME] do |t|
  in_deploy_dir
  puts "Restoring the snapshot to #{t.name}..."
  invoke_mina_indexer(
    "database", "restore",
    "--snapshot-file", SNAPSHOT_NAME,
    "--restore-dir", t.name
  ) || abort("Snapshot restore failed. Aborting.")
  puts "Restore complete."
end

desc "Create the database directory."
file DB_DIR => [RESTORE_DIR] do |t|
  in_deploy_dir

  # Send the currently running Indexer the shutdown command.
  #
  invoke_mina_indexer(
    "server", "shutdown"
  ) || puts("Shutting down (via command line) failed. Moving on.")

  # Maybe the shutdown worked, maybe it didn't. Either way:
  #
  puts "Give the process 1 second to clean up."
  sleep 1

  # Make the restored snapshot be the database directory.
  #
  FileUtils.rm_rf(DB_DIR)
  FileUtils.mv(RESTORE_DIR, DB_DIR)
end

desc "Deploy the mina-indexer."
file PID_FILE => [LOG_DIR, EXE, DB_DIR] do |t|
  in_deploy_dir

  # Send the currently running Indexer the shutdown command.
  #
  invoke_mina_indexer(
    "server", "shutdown"
  ) || puts("Shutting down (via command line) failed. Moving on.")

  # Maybe the shutdown worked, maybe it didn't. Either way:
  #
  puts "Give the process 1 second to clean up."
  sleep 1

  # Daemonize the EXE.
  #
  pid = fork
  if pid.nil?
    # I am the child. (The child gets a nil return value.)
    Process.setsid
    command_line = EXE +
      " server start" \
      " --log-level DEBUG" \
      " --ledger-cadence 1000" \
      " --database-dir #{DB_DIR}" \
      " --missing-block-recovery-exe #{SRC_TOP}/ops/recover-block" \
      " --missing-block-recovery-delay 11" \
      " --fetch-new-blocks-exe #{SRC_TOP}/ops/recover-block" \
      " --fetch-new-blocks-delay 7" \
      " --blocks-dir #{DEPLOY_DIR}/new-blocks" \
      " >> #{LOG_DIR}/out 2>> #{LOG_DIR}/err"
    puts "Spawning: #{command_line}"
    pid = spawn({"RUST_BACKTRACE" => "full"}, command_line)
    puts "Mina Indexer daemon spawned with PID #{pid}. Child exiting."
  end
  Process.detach pid
end
