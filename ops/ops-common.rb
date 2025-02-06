require "fileutils"
require "json"

BUILD_TYPE ||= "dev"
DEPLOY_TYPE ||= "test"
REV ||= `git rev-parse --short=8 HEAD`.strip
VOLUMES_DIR = ENV["VOLUMES_DIR"] || "/mnt"
DEPLOY_DIR ||= "#{VOLUMES_DIR}/mina-indexer-#{DEPLOY_TYPE}"
BASE_DIR ||= "#{DEPLOY_DIR}/#{REV}"

puts "Using base directory: #{BASE_DIR}"
FileUtils.mkdir_p(BASE_DIR)

SRC_TOP = `git rev-parse --show-toplevel`.strip
CURRENT = "#{BASE_DIR}/CURRENT"

# Port

def random_port
  rand 10_000..50_000
end

# Logs

LOGS_DIR = "#{BASE_DIR}/logs/"

def config_log_dir
  FileUtils.mkdir_p(LOGS_DIR)
end

# Snapshots

SNAPSHOTS_DIR = "#{BASE_DIR}/snapshots"

def config_snapshots_dir
  FileUtils.mkdir_p(SNAPSHOTS_DIR)
end

def snapshot_path(block_height)
  "#{SNAPSHOTS_DIR}/#{DB_VERSION}-#{block_height}.snapshot"
end

# Executable

EXE_DIR = "#{BASE_DIR}/bin"
EXE_SRC = if BUILD_TYPE == "nix"
  "#{SRC_TOP}/result/bin/mina-indexer"
else
  "#{SRC_TOP}/rust/target/debug/mina-indexer"
end
EXE = "#{EXE_DIR}/mina-indexer-#{REV}"

def invoke_mina_indexer(*args)
  system({'RUST_BACKTRACE' => 'full'}, EXE, *args)
end

def config_exe_dir
  FileUtils.mkdir_p(EXE_DIR)
  return if File.exist?(EXE)

  FileUtils.cp(EXE_SRC, EXE)
end

# Socket

SOCKET = "#{BASE_DIR}/mina-indexer.sock"

def wait_for_socket(wait_interval)
  wait_seconds = 0
  until File.exist?(SOCKET)
    puts "Waited #{wait_seconds}s for #{SOCKET}..."
    sleep wait_interval
    wait_seconds += wait_interval
  end
end

# Ledgers

LEDGERS_DIR = "#{DEPLOY_DIR}/staking-ledgers"

def fetch_ledgers
  system("#{SRC_TOP}/ops/download-staking-ledgers.rb", LEDGERS_DIR) ||
    abort("Something went wrong with staking ledger downloads.")
end

# Blocks

def blocks_dir(block_height)
  "#{DEPLOY_DIR}/blocks-#{block_height}"
end

def get_blocks(block_height)
  system(
    "#{SRC_TOP}/ops/download-mina-blocks.rb",
    "1", # start block
    block_height.to_s, # end block
    blocks_dir(block_height)
  ) || abort("Downloading Mina blocks failed.")
end

# Database directory

v = JSON.parse(`#{EXE_SRC} database version --json`)
DB_VERSION_JSON = v
DB_VERSION = "#{v["major"]}.#{v["minor"]}.#{v["patch"]}"

def db_dir(block_height)
  "#{BASE_DIR}/db-#{DB_VERSION}-#{block_height}"
end

# Deploy

def idxr_cleanup(which, rev)
  if which == "one"
    FileUtils.rm_rf(BASE_DIR)
  elsif which == "all"
    FileUtils.rm_rf(Dir.glob("#{DEPLOY_DIR}/*"))
  end
end

def idxr_shutdown(rev)
  mina_indexer = "mina-indexer-#{rev}"
  puts "Shutting down #{mina_indexer}"
  idxr_shutdown_via_socket(EXE, "#{BASE_DIR}/mina-indexer.sock")
end

# Shutdown

def idxr_shutdown_via_socket(exe, socket)
  # Attempt a regular shutdown if the socket is present.
  return unless File.exist?(socket)

  unless system(
    exe,
    "--socket", socket,
    "server", "shutdown"
  )
    warn("Shutdown failed despite #{socket} existing.")
    return
  end

  sleep 1 # Give it a chance to shut down.
  FileUtils.rm_f(socket)
end
