#! /usr/bin/env -S ruby -w

# show mina-indexer PID(s)
if ARGV == ["pids", "show"]
  system("pgrep", "mina-indexer") || puts("No running indexer")
  exit 0
end

DEPLOY_TYPE = ARGV[0] # 'dev', 'prod', or 'test'
BUILD_TYPE = "debug"

VOLUMES_DIR = ENV["VOLUMES_DIR"] || "/mnt"
BASE_DIR = "#{VOLUMES_DIR}/mina-indexer-#{DEPLOY_TYPE}"

require "fileutils"
require "#{__dir__}/helpers" # Requires BASE_DIR & BUILD_TYPE

DEV_DIR = "#{BASE_DIR}/rev-#{REV}" # Only makes sense for DEPLOY_TYPE == 'dev'

#
# Clean
#

def clean_dev(type)
  if type == "dev"
    if ARGV[2] == "all"
      puts "Removing all dev rev directories"
      FileUtils.rm_rf(Dir.glob("#{BASE_DIR}/rev-*"))
    elsif Dir.exist? DEV_DIR
      puts "Removing #{DEV_DIR}"
      FileUtils.rm_rf(Dir.glob(DEV_DIR))
    end
    exit 0
  end
end

def clean(type)
  if type == "prod" || type == "test"
    idxr_cleanup(ARGV.last)
    exit 0
  end
end

# clean up directories
if ARGV[1] == "clean"
  clean_dev(DEPLOY_TYPE)
  clean(DEPLOY_TYPE)
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
