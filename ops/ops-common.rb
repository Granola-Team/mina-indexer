require "fileutils"
require "json"

BUILD_TYPE ||= "nix" # standard:disable Lint/OrAssignmentToConstant
DEPLOY_TYPE ||= "test" # standard:disable Lint/OrAssignmentToConstant
REV ||= `git rev-parse --short=8 HEAD`.strip # standard:disable Lint/OrAssignmentToConstant
VOLUMES_DIR = ENV["VOLUMES_DIR"] || "/mnt"
DEPLOY_DIR ||= "#{VOLUMES_DIR}/mina-indexer-#{DEPLOY_TYPE}" # standard:disable Lint/OrAssignmentToConstant

def base_dir_from_rev(rev)
  "#{DEPLOY_DIR}/#{rev}"
end

BASE_DIR ||= base_dir_from_rev(REV) # standard:disable Lint/OrAssignmentToConstant

def config_base_dir
  puts "Using base directory: #{BASE_DIR}"
  FileUtils.mkdir_p(BASE_DIR)
end

SRC_TOP ||= `git rev-parse --show-toplevel`.strip # standard:disable Lint/OrAssignmentToConstant
CURRENT = "#{DEPLOY_DIR}/CURRENT"

# Port

def random_port
  rand 10_000..50_000
end

# Logs

LOGS_DIR = "#{BASE_DIR}/logs"

def config_log_dir
  FileUtils.mkdir_p(LOGS_DIR)
end

# Snapshots

SNAPSHOTS_DIR = "#{BASE_DIR}/snapshots"

def config_snapshots_dir
  FileUtils.mkdir_p(SNAPSHOTS_DIR)
end

def snapshot_path(block_height)
  "#{SNAPSHOTS_DIR}/mina-#{block_height}.snapshot"
end

# Executable

EXE_DIR = "#{BASE_DIR}/bin"
EXE_SRC = if BUILD_TYPE == "nix"
  "#{SRC_TOP}/result/bin/mina-indexer"
else
  "#{SRC_TOP}/rust/target/debug/mina-indexer"
end
EXE = "#{EXE_DIR}/mina-indexer-#{REV}"

def invoke_mina_indexer(*)
  system({"RUST_BACKTRACE" => "full"}, EXE, *)
end

def config_exe_dir
  FileUtils.mkdir_p(EXE_DIR)
  return if File.exist?(EXE)

  FileUtils.cp(EXE_SRC, EXE)
end

# Socket

def socket_from_rev(rev)
  "#{base_dir_from_rev(rev)}/mina-indexer.sock"
end

SOCKET = socket_from_rev(REV)

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

MASTER_BLOCKS_DIR = "#{DEPLOY_DIR}/blocks"

def blocks_dir(block_height)
  "#{DEPLOY_DIR}/blocks-#{block_height}"
end

def stage_blocks(end_height, start_height = 1, network = "mainnet", dest = "")
  dest = blocks_dir(end_height) if dest == ""

  # If start_height is not 1, then we assume that even if the destination
  # exists, then it does not contain all 1..end_height contiguous blocks.
  #
  # If the start height is 1, then we assume that if the destination exists,
  # then it does contain all 1..end_height contiguous blocks.
  #
  if start_height != 1 || !File.exist?(dest)

    # Ensure that the blocks are present in the main blocks directory.
    #
    system("#{__dir__}/download-mina-blocks.rb",
      start_height.to_s,
      end_height.to_s,
      MASTER_BLOCKS_DIR) || abort("Failure of download-mina-blocks.rb")

    FileUtils.mkdir_p(dest)

    print "Staging #{network} blocks #{start_height} to #{end_height} into #{dest}... "

    # Format of file is: "#{MASTER_BLOCKS_DIR}/#{network}-#{block_height}-#{hash}.json"
    #
    Dir["#{MASTER_BLOCKS_DIR}/#{network}-*.json"].each do |block_file|
      /#{MASTER_BLOCKS_DIR}\/#{network}-(.*)-.*\.json/ =~ block_file
      height = $1
      if height.to_i.between?(start_height, end_height)
        # Hard link the correct block files into the destination directory.
        target = "#{dest}/#{File.basename(block_file)}"
        unless File.exist?(target)
          File.link(block_file, target)
        end
      end
    end
    puts
  end
end

# Database directory

def db_dir(block_height)
  "#{BASE_DIR}/db-#{block_height}"
end

# Deploy

def idxr_cleanup(which)
  if which == "one"
    puts "Removing #{BASE_DIR}"
    FileUtils.rm_rf(BASE_DIR)
  elsif which == "all"
    Dir.glob("#{DEPLOY_DIR}/*").each do |path|
      # remove everything except blocks dirs & blocks.list
      if /blocks.*/.match(File.basename(path)).nil?
        puts "Removing #{path}"
        FileUtils.rm_rf(path)
      end
    end
  end
end

def idxr_shutdown
  puts "Shutting down mina-indexer-#{File.basename(BASE_DIR)}"
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
