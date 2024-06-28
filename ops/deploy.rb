#! /usr/bin/env -S ruby -w

DEPLOY_TYPE = ARGV[0]   # 'test' or 'prod'
BLOCKS_COUNT = ARGV[1]  #  number of blocks to deploy

VOLUMES_DIR = ENV["VOLUMES_DIR"] || '/mnt'
BASE_DIR = VOLUMES_DIR + '/mina-indexer-' + DEPLOY_TYPE

require __dir__ + '/helpers'  # Expects BASE_DIR to be defined

unless File.exist?(BASE_DIR)
  abort "Error: #{BASE_DIR} must exist to perform the deployment."
end
puts "Deploying (#{DEPLOY_TYPE}) with #{BLOCKS_COUNT} blocks."

# Configure the directories as needed.
#
createBaseDir
configExecDir
configLogDir
configSnapshotDir
get_blocks BLOCKS_COUNT
getLedgers

# Terminate the current version, if any.
#
if File.exist? CURRENT
  current = File.read(CURRENT)
  puts "Shutting down #{current}..."
  system(
    EXE,
    "--socket", "#{BASE_DIR}/mina-indexer-#{current}.socket",
    "shutdown"
  ) || puts('Shutting down (via command line and socket) failed. Moving on.')

  # Maybe the shutdown worked, maybe it didn't. Either way, give the process
  # a second to clean up.
  sleep 1
end

# Now, we take over.
#
File::write CURRENT, REV

if DEPLOY_TYPE == 'test'
  PORT = randomPort
  pid = spawn EXE +
    " --socket #{SOCKET} " +
    " server start" +
    " --log-level DEBUG" +
    " --ledger-cadence 5000" +
    " --web-port #{PORT.to_s}" +
    " --database-dir #{db_dir(BLOCKS_COUNT)}" +
    " --blocks-dir #{blocks_dir(BLOCKS_COUNT)}" +
    " --staking-ledgers-dir #{LEDGERS_DIR}" +
    " >> #{LOGS_DIR}/out 2>> #{LOGS_DIR}/err"
  waitForSocket(10)
  system(
    EXE,
    '--socket', SOCKET,
    'create-snapshot', '--output-dir', snapshot_dir(BLOCKS_COUNT)
  ) || abort('Snapshot creation failed. Aborting.')
  puts 'Skipping replay. It does not work. See issue #1196.'
  system(
    EXE,
    "--socket", SOCKET,
    "shutdown"
  ) || puts('Shutdown failed after snapshot.')
  Process.wait(pid)
  File::delete(CURRENT)
else
  # Daemonize the EXE.
  #
  pid = fork
  if pid
    # Then I am the parent. Register disinterest in the child PID.
    Process::detach pid
    puts "Session dispatched with PID #{pid}. Parent exiting."
  else
    # I am the child. (The child gets a nil return value.)
    Process.setsid
    pid = spawn EXE +
      " --socket #{SOCKET} " +
      " server start" +
      " --log-level DEBUG" +
      " --ledger-cadence 5000" +
      " --web-hostname 0.0.0.0" +
      " --database-dir #{db_dir(BLOCKS_COUNT)}" +
      " --blocks-dir #{blocks_dir(BLOCKS_COUNT)}" +
      " --staking-ledgers-dir #{LEDGERS_DIR}" +
      " >> #{LOGS_DIR}/out 2>> #{LOGS_DIR}/err"
    Process::detach pid
    puts "Mina Indexer daemon dispatched with PID #{pid}. Child exiting."
  end
end
