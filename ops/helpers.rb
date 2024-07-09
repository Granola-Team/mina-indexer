# frozen_string_literal: true

require 'fileutils'
require 'json'

# NOTE: expects BASE_DIR to be defined.

# Constants

SRC_TOP = `git rev-parse --show-toplevel`.strip
REV = `git rev-parse --short=8 HEAD`.strip
CURRENT = "#{BASE_DIR}/CURRENT"

# Port

def random_port
  rand 10_000..50_000
end

# Base directory

def config_base_dir
  FileUtils.mkdir_p BASE_DIR
end

# Logs

LOGS_DIR = "#{BASE_DIR}/logs/#{REV}"

def config_log_dir
  FileUtils.mkdir_p LOGS_DIR
end

# Snapshots

SNAPSHOTS_DIR = "#{BASE_DIR}/snapshots"

def config_snapshots_dir
  FileUtils.mkdir_p(SNAPSHOTS_DIR)
end

def snapshot_path(block_height)
  "#{SNAPSHOTS_DIR}/#{DB_VERSION}-#{block_height}-#{REV}.snapshot"
end

# Executable

EXE_DIR = "#{BASE_DIR}/bin"
EXE_SRC = "#{SRC_TOP}/result/bin/mina-indexer"
EXE = "#{EXE_DIR}/mina-indexer-#{REV}"

def config_exe_dir
  FileUtils.mkdir_p EXE_DIR
  return if File.exist?(EXE)

  FileUtils.cp EXE_SRC, EXE
end

# Socket

SOCKET = "#{BASE_DIR}/mina-indexer-#{REV}.socket"

def wait_for_socket(wait_interval)
  wait_seconds = 0
  until File.exist?(SOCKET)
    puts "Waited #{wait_seconds} s for #{SOCKET}..."
    sleep wait_interval
    wait_seconds += wait_interval
  end
end

# Ledgers

LEDGERS_DIR = "#{BASE_DIR}/staking-ledgers"

def fetch_ledgers
  system("#{SRC_TOP}/ops/download-staking-ledgers", LEDGERS_DIR) ||
    abort('Something went wrong with staking ledger downloads.')
end

# Blocks

def blocks_dir(block_height)
  "#{BASE_DIR}/blocks-#{block_height}"
end

def get_blocks(block_height)
  system(
    "#{SRC_TOP}/ops/download-mina-blocks.rb",
    '1', # start block
    block_height.to_s, # end block
    blocks_dir(block_height)
  ) || abort('Downloading Mina blocks failed.')
end

# Database directory

v = JSON.parse(`#{EXE_SRC} database version --json`)
DB_VERSION_JSON = v
DB_VERSION = "#{v['major']}.#{v['minor']}.#{v['patch']}"

def db_dir(block_height)
  "#{BASE_DIR}/db/#{DB_VERSION}-#{block_height}"
end
