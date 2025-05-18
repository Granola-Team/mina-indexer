#!/usr/bin/env -S ruby -w

# show mina-indexer PID(s)
if ARGV == ["pids", "show"]
  system("pgrep", "mina-indexer") || puts("No running indexer")
  exit 0
end

# kill mina-indexer PID(s)
if ARGV.first == "kill"
  pids = `pgrep mina-indexer`.chomp

  if pids.empty?
    puts "No running indexer PID(s)"
    return
  end

  puts pids
  pids = pids.split("\n")
  pids.map { |pid| system("kill", "-9", pid) }
end

DEPLOY_TYPE = ARGV[0] # Only accepts 'test'
BUILD_TYPE = "dev"

require "#{__dir__}/ops-common"

#
# Clean
#

# clean up directories
if ARGV[1] == "clean"
  idxr_cleanup(ARGV.last)
  exit 0
end

#
# Show
#

def show(type, which)
  dir = if which == "one"
    BASE_DIR
  elsif which == "all"
    deploy_dir
  end

  if type == "test"
    if Dir.exist?(dir)
      system("ls", "-l", dir)
      exit 0
    end
  end
end

# show directories
if ARGV[1] == "show"
  show(DEPLOY_TYPE, ARGV.last)
  exit 0
end

#
# Shutdown
#

# Check if we're shutting down a running indexer
#
if ARGV.length == 2 && ARGV.last == "shutdown"
  idxr_shutdown
  exit 0
end
