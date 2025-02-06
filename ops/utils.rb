#!/usr/bin/env -S ruby -w

# -*- mode: ruby -*-

# show mina-indexer PID(s)
if ARGV == ["pids", "show"]
  system("pgrep", "mina-indexer") || puts("No running indexer")
  exit 0
end

DEPLOY_TYPE = ARGV[0] # 'dev', 'prod', or 'test'
BUILD_TYPE = "debug"

require "#{__dir__}/ops-common" # Requires BASE_DIR & BUILD_TYPE

#
# Clean
#

def clean_dev(type)
  if type == "dev"
    idxr_cleanup(ARGV.last)
    exit 0
  end
end

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
  clean_dev(DEPLOY_TYPE)
  clean_prod(DEPLOY_TYPE)
  clean_test(DEPLOY_TYPE)
  exit 0
end

#
# Show
#

def show_dev(type)
  if type == "dev"
    if Dir.exist? DEV_DIR
      system("ls", "-l", DEV_DIR)
      exit 0
    end
  end
end

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
  show_dev(DEPLOY_TYPE)
  show(DEPLOY_TYPE)
  exit 0
end

#
# Shutdown
#

# Check if we're shutting down a running indexer
#
if ARGV.length == 3 && ARGV[-2] == "shutdown"
  idxr_shutdown(ARGV.last)
  exit 0
end
