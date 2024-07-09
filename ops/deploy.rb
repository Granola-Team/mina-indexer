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
config_base_dir
config_exe_dir
config_log_dir
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
  unless File.exist?(db_dir(BLOCKS_COUNT))
    puts 'Initiating database creation...'
    system(
      EXE,
      'database', 'create',
      '--log-level', 'DEBUG',
      '--ledger-cadence', '5000',
      '--database-dir', db_dir(BLOCKS_COUNT),
      '--blocks-dir', blocks_dir(BLOCKS_COUNT),
      '--staking-ledgers-dir', LEDGERS_DIR,
    ) || abort('database creation failed')
    puts 'Database creation succeeded.'
  end

  puts 'Restarting server...'
  PORT = randomPort
  pid = spawn EXE +
    " --socket #{SOCKET} " +
    " server start" +
    " --log-level DEBUG" +
    " --ledger-cadence 5000" +
    " --blocks-dir #{blocks_dir(BLOCKS_COUNT)}" +
    " --staking-ledgers-dir #{LEDGERS_DIR}" +
    " --web-port #{PORT.to_s}" +
    " --database-dir #{db_dir(BLOCKS_COUNT)}" +
    " >> #{LOGS_DIR}/out 2>> #{LOGS_DIR}/err"
  waitForSocket(10)
  puts 'Server restarted.'

  puts "Creating snapshot at #{snapshot_path(BLOCKS_COUNT)}..."
  config_snapshots_dir
  system(
    EXE,
    '--socket', SOCKET,
    'database', 'snapshot',
    '--output-path', snapshot_path(BLOCKS_COUNT)
  ) || abort('Snapshot creation failed. Aborting.')
  puts 'Snapshot complete.'

  puts 'Initiating shutdown...'
  system(
    EXE,
    "--socket", SOCKET,
    "shutdown"
  ) || puts('Shutdown failed after snapshot.')
  Process.wait(pid)

# TODO: make self-check work
#
#  puts 'Initiating self-check...'
#  pid = spawn EXE +
#    " --socket #{SOCKET} " +
#    " server start" +
#    " --self-check" +
#    " --log-level DEBUG" +
#    " --ledger-cadence 5000" +
#    " --web-port #{PORT.to_s}" +
#    " --database-dir #{db_dir(BLOCKS_COUNT)}" +
#    " --blocks-dir #{blocks_dir(BLOCKS_COUNT)}" +
#    " --staking-ledgers-dir #{LEDGERS_DIR}" +
#    " >> #{LOGS_DIR}/out 2>> #{LOGS_DIR}/err"
#  waitForSocket(10)
#  puts 'Self-check complete.'
#
#  puts 'Initiating shutdown...'
#  system(
#    EXE,
#    "--socket", SOCKET,
#    "shutdown"
#  ) || puts('Shutdown failed after snapshot.')
#  Process.wait(pid)
#  puts 'Shutdown complete.'

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
