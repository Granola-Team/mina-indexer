require "fileutils"
require "open3"

GIT_COMMIT_HASH = ENV["GIT_COMMIT_HASH"].freeze
VOLUMES_DIR = ENV.fetch("VOLUMES_DIR", "/mnt")
BASE_DIR = File.join(File.join(VOLUMES_DIR, "mina-indexer-test"), GIT_COMMIT_HASH).freeze
PID_FILE = File.join(BASE_DIR, "idxr_pid").freeze
SOCKET_FILE = File.join(BASE_DIR, "mina-indexer.sock").freeze
BLOCKS_DIR = File.join(BASE_DIR, "blocks").freeze
DATABASE_DIR = File.join(BASE_DIR, "database").freeze
STAKING_LEDGERS_DIR = File.join(BASE_DIR, "staking-ledgers").freeze
STAKING_LEDGERS_URL = "https://staking-ledgers.minasearch.com"
V2_GENESIS_STATE_HASH = "3NK4BpDSekaqsG6tx8Nse2zJchRft2JpnbvMiog55WCr5xJZaKeP".freeze

# Set environment variables
ENV["RUST_BACKTRACE"] = "full"

# Helper methods
def find_ephemeral_port
  rand(49152..65535)
end

def wait_for_socket(max_retries = 250)
  max_retries = max_retries.to_i

  max_retries.times do |i|
    return true if File.socket?(SOCKET_FILE)
    puts "Sleeping (#{i + 1})..."
    sleep(1)
  end

  false
end

def read_pid_file
  File.read(PID_FILE).strip if File.exist?(PID_FILE)
end

def run(cmd, *args, dir: __dir__)
  success = system(cmd, *args, chdir: dir)
  abort "Command failed: #{cmd} #{args.join(" ")}" unless success
  success
end

def process_running?(pid)
  return false unless pid
  run("ps -p #{pid} > /dev/null 2>&1")
end

task bin: ["bin:list"]

namespace :bin do
  desc "List available tasks"
  task :list do
    run("rake -T bin")
  end

  desc "Run Indexer with specified arguments"
  task :run, [:idxr_bin, *:args] do |_, args|
    idxr_bin = args[:idxr_bin]

    # Get all arguments including the first one
    all_args = []
    all_args << args[:args] if args[:args]
    all_args.concat(args.extras)

    # Join with spaces for command line
    command_args = all_args.join(" ")

    Dir.chdir(BASE_DIR) do
      run("#{idxr_bin} --socket #{SOCKET_FILE} #{command_args}", dir: BASE_DIR)
    end
  end

  desc "Shutdown the Indexer server"
  task :shutdown, [:idxr_bin] do |_, args|
    idxr_bin = args[:idxr_bin]

    puts "Shutting down Mina Indexer."

    if File.exist?(PID_FILE)
      pid = File.read(PID_FILE).strip

      if system("ps -p #{pid} > /dev/null 2>&1")
        puts "Shutting down Mina Indexer..."

        # Try graceful shutdown first
        begin
          run("#{idxr_bin} --socket #{SOCKET_FILE} server shutdown", dir: BASE_DIR)

          # Wait for process to exit
          30.times do
            break unless system("ps -p #{pid} > /dev/null 2>&1")
            sleep 1
          end
        rescue => e
          puts "Error during graceful shutdown: #{e.message}"
        end

        # Force kill if still running
        if system("ps -p #{pid} > /dev/null 2>&1")
          puts "Force killing process..."
          system("kill -9 #{pid}")
        end
      else
        puts "Process #{pid} not running."
      end

      # Clean up PID file
      FileUtils.rm_f(PID_FILE)
    else
      puts "No PID file found at #{PID_FILE}"
    end

    # Clean up socket file
    FileUtils.rm_f(SOCKET_FILE) if File.exist?(SOCKET_FILE)
  end

  desc "Find an ephemeral port"
  task :ephemeral_port do
    puts find_ephemeral_port
  end

  desc "Wait for socket with timeout"
  task :wait_for_socket, [:max_retries] do |_, args|
    max_retries = args[:max_retries] || 250
    abort "Socket not available after #{max_retries} retries" unless wait_for_socket(max_retries)
  end

  desc "Wait indefinitely for socket"
  task :wait_forever_for_socket do
    until File.socket?(SOCKET_FILE)
      puts "Sleeping 10s..."
      sleep(10)
    end
  end

  desc "Create an Indexer database"
  task :database_create, [:idxr_bin, *:args] do |_, args|
    idxr_bin = args[:idxr_bin]

    # Get all arguments including the first one
    all_args = []
    all_args << args[:args] if args[:args]
    all_args.concat(args.extras)

    FileUtils.mkdir_p(BASE_DIR)

    cmd_args = [
      idxr_bin,
      "--socket", SOCKET_FILE,
      "database", "create",
      "--blocks-dir", BLOCKS_DIR,
      "--staking-ledgers-dir", STAKING_LEDGERS_DIR,
      "--database-dir", DATABASE_DIR
    ]
    cmd_args.concat(all_args)

    Dir.chdir(BASE_DIR) do
      run(cmd_args.join(" "), dir: BASE_DIR)
    end
  end

  desc "Start the Indexer server and wait for socket (with timeout)"
  task :start, [:idxr_bin, *:args] do |_, args|
    idxr_bin = args[:idxr_bin]

    # Get all arguments including the first one
    all_args = []
    all_args << args[:args] if args[:args]
    all_args.concat(args.extras)

    FileUtils.mkdir_p(BASE_DIR)

    cmd = [
      idxr_bin,
      "--socket", SOCKET_FILE,
      "server", "start"
    ]
    cmd.concat(all_args)

    Dir.chdir(BASE_DIR) do
      run("#{cmd.join(" ")} & echo $! > #{PID_FILE}", dir: BASE_DIR)
    end

    sleep 2  # Add a small delay before checking for socket
    Rake::Task["bin:wait_for_socket"].invoke
  end

  desc "Start the Indexer server with an ephemeral port"
  task :_start, [:idxr_bin, *:args] do |_, args|
    idxr_bin = args[:idxr_bin]

    # Get all arguments including the first one
    all_args = []
    all_args << args[:args] if args[:args]
    all_args.concat(args.extras)

    port = find_ephemeral_port

    start_args = [
      "--web-port", port.to_s,
      "--blocks-dir", BLOCKS_DIR,
      "--staking-ledgers-dir", STAKING_LEDGERS_DIR,
      "--database-dir", DATABASE_DIR
    ]
    start_args.concat(all_args)

    Rake::Task["bin:start"].invoke(idxr_bin, *start_args)
  end

  desc "Create a v1 database and start server with this database"
  task :start_v1, [:idxr_bin, *:args] do |_, args|
    idxr_bin = args[:idxr_bin]

    # Get all arguments including the first one
    all_args = []
    all_args << args[:args] if args[:args]
    all_args.concat(args.extras)

    puts "Creating v1 mina Indexer database"
    Rake::Task["bin:database_create"].reenable
    Rake::Task["bin:database_create"].invoke(idxr_bin, *all_args)

    puts "Starting mina Indexer server from v1 database"
    Rake::Task["bin:_start"].reenable
    Rake::Task["bin:_start"].invoke(idxr_bin, *all_args)
  end

  desc "Create a v2 database and start server with this database"
  task :start_v2, [:idxr_bin] do |_, args|
    idxr_bin = args[:idxr_bin]

    puts "Creating v2 mina Indexer database"
    Rake::Task["bin:database_create"].reenable
    Rake::Task["bin:database_create"].invoke(idxr_bin, "--genesis-hash", V2_GENESIS_STATE_HASH)

    puts "Starting mina Indexer server from v2 database"
    Rake::Task["bin:_start"].reenable
    Rake::Task["bin:_start"].invoke(idxr_bin, "--genesis-hash", V2_GENESIS_STATE_HASH)
  end

  desc "Stage blocks (up to `block_height`), download staking ledgers, create a v2 database, and start server with this database"
  task :stage_and_start_v2, [:idxr_bin, :block_height, :web_port, *:args] do |_, args|
    require "net/http"
    require "uri"

    idxr_bin = args[:idxr_bin]
    block_height = args[:block_height]
    # Set default web_port if not provided
    web_port = (args[:web_port].nil? || args[:web_port].empty?) ? 8080 : args[:web_port].to_i

    # Get all arguments including the first one
    all_args = []
    all_args << args[:args] if args[:args]
    all_args.concat(args.extras)

    # Create directories if they don't exist
    FileUtils.mkdir_p(BLOCKS_DIR)
    FileUtils.mkdir_p(STAKING_LEDGERS_DIR)

    # List of staking ledger filenames to download
    staking_ledger_files = [
      "mainnet-0-jxsAidvKvEQJMC7Z2wkLrFGzCqUxpFMRhAj4K5o49eiFLhKSyXL.json",
      "mainnet-1-jwgzfxD5rEnSP3k4UiZu2569FfhJ1SRUvabfTz21e4btwBHg3jq.json",
      "mainnet-2-jw8dq1FtwJxbwqU1aYCxjY98fE21CqMDMynsXzRwAHAvM6yhx5A.json",
      "mainnet-3-jwPSjgLj5AsJtA1oTqMasQrxpeZx7pmGy5HKnAAhPBC4tYYvEj5.json"
    ]

    # Create threads for parallel execution
    threads = []

    # Thread for block staging
    threads << Thread.new do
      Rake::Task["stage_blocks:v2"].invoke(block_height, BLOCKS_DIR)
    end

    # Threads for downloading each staking ledger in parallel
    staking_ledger_files.each do |filename|
      threads << Thread.new(filename) do |file|
        url = "#{STAKING_LEDGERS_URL}/#{file}"
        output_path = File.join(STAKING_LEDGERS_DIR, file)

        puts "Downloading staking ledger from #{url}"
        begin
          uri = URI.parse(url)
          response = Net::HTTP.get_response(uri)

          if response.is_a?(Net::HTTPSuccess)
            File.binwrite(output_path, response.body)
          else
            puts "Error downloading #{url}: HTTP #{response.code} - #{response.message}"
          end
        rescue => e
          puts "Error downloading #{url}: #{e.message}"
        end
      end
    end

    # Wait for all operations to complete
    threads.each(&:join)
    puts "All parallel operations completed"

    Rake::Task["bin:database_create"].reenable
    Rake::Task["bin:database_create"].invoke(idxr_bin, "--genesis-hash", V2_GENESIS_STATE_HASH)

    start_args = [
      "--web-port", web_port.to_s,
      "--blocks-dir", BLOCKS_DIR,
      "--staking-ledgers-dir", STAKING_LEDGERS_DIR,
      "--database-dir", DATABASE_DIR,
      "--genesis-hash", V2_GENESIS_STATE_HASH
    ]
    start_args.concat(all_args)

    Rake::Task["bin:start"].reenable
    Rake::Task["bin:start"].invoke(idxr_bin, *start_args)
  end
end
