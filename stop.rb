#! /usr/bin/env -S ruby -w

require './helpers'

if File.exist? CURRENT
  current = File.read(CURRENT)
end

if ! current
  abort "There is no #{CURRENT} file, or it is unusable."
end

dbg `ls -laR ~/mina-indexer/#{VERSION}`

puts "Shutting down #{current}..."
success = system EXE +
  " --socket #{HOME_DIR}/#{current}/mina-indexer.socket" +
  " shutdown"
if success
  # Maybe the shutdown worked, maybe it didn't. Either way, give the process a
  # second to clean up.
  puts "Success!"
  sleep 1
else
  abort "Shutting down (via command line and socket) failed."
end
