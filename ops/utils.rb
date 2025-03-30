#!/usr/bin/env -S ruby -w

# -*- mode: ruby -*-

# show mina-indexer PID(s)
if ARGV == ["pids", "show"]
  system("pgrep", "mina-indexer") || puts("No running indexer")
  exit 0
end

DEPLOY_TYPE = ARGV[0] # 'prod' or 'test'
BUILD_TYPE = "dev"

require "#{__dir__}/ops-common"

#
# Clean
#

def clean_prod(type)
  if type == "prod"
    idxr_cleanup(ARGV.last)
    exit 0
  end
end

def clean_test(type)
  if type == "test"
    idxr_cleanup("one")
    exit 0
  end
end

# clean up directories
if ARGV[1] == "clean"
  clean_prod(DEPLOY_TYPE)
  clean_test(DEPLOY_TYPE)
  exit 0
end

#
# Show
#

def show(type)
  if type == "prod" || type == "test"
    if Dir.exist? BASE_DIR
      system("ls", "-l", BASE_DIR)
      exit 0
    end
  end
end

# show directories
if ARGV[1] == "show"
  show(DEPLOY_TYPE)
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
