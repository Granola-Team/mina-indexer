#! /usr/bin/env -S ruby -w

BASE_DIR = ARGV[0]
MAGNITUDE = ARGV[1]

require __dir__ + '/helpers'  # Expects BASE_DIR and MAGNITUDE to be defined

# First check to see if we are trying to redeploy the same revision that is
# currently running.
#
if File.exist? CURRENT
  current = File.read(CURRENT)
  if current == REV
    abort "Redeploying the same version is not supported. Exiting."
  else
    puts "Deploying with intention to kill #{current} upon success."
  end
end

# Configure the directories as needed.
#
createBaseDir
configExecDir
configLogDir
getBlocks MAGNITUDE
getLedgers

# Terminate the current version, if any.
#
if current
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
    " --database-dir #{DB_DIR}" +
    " --blocks-dir #{BLOCKS_DIR}" +
    " --staking-ledgers-dir #{LEDGERS_DIR}" +
    " >> #{LOGS_DIR}/out 2>> #{LOGS_DIR}/err"
  Process::detach pid
  puts "Mina Indexer daemon dispatched with PID #{pid}. Child exiting."
end
