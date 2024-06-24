#! /usr/bin/env -S ruby -w

require './helpers'

# First check to see if we are trying to redeploy the same version that is
# currently running.
if File.exist? CURRENT
  current = File.read(CURRENT)
  if current == VERSION
    abort "Redeploying the same version is not supported. Exiting."
  else
    puts "Deploying with intention to kill #{current} upon success."
  end
end

createBaseDir
configLogDir
configExecDir
dbg `ls -laR ~/mina-indexer/#{VERSION}`

# getLedgers
# dbg `ls -laR ~/mina-indexer/#{VERSION}`
# getBlocks 1
# dbg `ls -laR ~/mina-indexer/#{VERSION}`

# Perform the initial ingestion.
#
Dir.chdir BASE_DIR
port = randomPort
pid = spawn EXE +
  " --socket #{SOCKET}" +
  " server start" +
  " --web-port #{port}" +
  " --log-level DEBUG" +
  " --database-dir #{DB_DIR}" +
  " --blocks-dir #{BLOCKS_DIR}" +
  " --staking-ledgers-dir #{LEDGERS_DIR}" +
  " --ledger-cadence 5000" +
  " > #{LOGS_DIR}/out 2> #{LOGS_DIR}/err"
waitSeconds = 0
until File.exist?(SOCKET) do
  puts "Waiting (#{waitSeconds} s total) for #{SOCKET}..."
  sleep 10
  waitSeconds += 10
end
puts "Socket (#{SOCKET}) was created."
puts "Shutting down the ingester via #{SOCKET}..."
success = system EXE + " --socket #{SOCKET} shutdown"
if success
  puts "... ingester successfully shut down."
else
  abort "The shut down (via #{SOCKET} failed."
end
Process::wait pid

# The ingestion should have completed at this point.
# Terminate the current version, if any.

if current
  puts "Shutting down #{current}..."
  success = system EXE +
    " --socket #{HOME_DIR}/#{current}/mina-indexer.socket" +
    " shutdown"
  if ! success
    puts "Shutting down (via command line and socket) failed. Moving on."
  end

  # Maybe the shutdown worked, maybe it didn't. Either way, give the process
  # a second to clean up.
  sleep 1

  # Delete the directory used by current.
  FileUtils.rm_rf HOME_DIR "/#{current}"
end

# Now, we take over.
File::write CURRENT, VERSION

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
    " --socket #{SOCKET} server start" +
    " --log-level DEBUG" +
    " --web-hostname 0.0.0.0" +
    " --database-dir #{DB_DIR}" +
    " --blocks-dir #{BLOCKS_DIR}" +
    " --staking-ledgers-dir #{LEDGERS_DIR}" +
    " >> #{LOGS_DIR}/out 2>> #{LOGS_DIR}/err"
  Process::detach pid
  puts "Mina Indexer daemon dispatched with PID #{pid}. Child exiting."
end
