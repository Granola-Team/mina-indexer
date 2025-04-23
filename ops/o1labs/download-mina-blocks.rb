#!/usr/bin/env -S ruby -w

require "etc"
require "fileutils"
require "json"
require "net/http"
require "optparse"
require "socket"
require "uri"

require_relative "../update-pcbs"

BASE_URL = "https://storage.googleapis.com/mina_network_block_data"
BLOCK_THREADS = Etc.nprocessors * 4

PCB_UPDATER = PcbUpdater.new

# Force IPv4 by monkey patching TCPSocket. This is required because the IPv6
# address is sometimes used if so directed by DNS resolution, and yet the IPv6
# address does not work for 'storage.googleapis.com'.
#
class << TCPSocket
  alias_method :original_open, :open

  def open(host, port, *)
    # Force IPv4 address family
    addrs = Socket.getaddrinfo(host, port, Socket::AF_INET)
    addr = addrs.first
    original_open(addr[3], port, *)
  end
end

def parse_arguments
  options = {
    dir: ".",
    state_hash: nil,
    command: :range  # Default to original behavior
  }

  parser = OptionParser.new do |opts|
    opts.banner = "Usage: download-mina-blocks [options] <command> <args...>"

    opts.on("-d", "--dir DIRECTORY", "Directory to save blocks (default: current directory)") do |dir|
      options[:dir] = dir
    end

    opts.on("-h", "--help", "Show this help message") do
      puts opts
      puts "\nCommands:"
      puts "  block HEIGHT STATE_HASH  Download a specific block"
      puts "  blocks HEIGHT            Download all blocks at a specific height"
      puts "  MIN_HEIGHT MAX_HEIGHT                  Download all blocks between heights (original behavior)"
      exit
    end
  end

  parser.parse!

  # Check if using a rake task
  if ["block", "blocks"].include?(ARGV[0])
    command = ARGV[0]
    height = ARGV[1]

    if command.nil? || height.nil?
      puts parser
      exit 1
    end

    case command
    when "block"
      state_hash = ARGV[2]
      if state_hash.nil?
        puts "Error: STATE_HASH is required for block command"
        puts parser
        exit 1
      end
      options[:state_hash] = state_hash
      options[:command] = :single
      options[:height] = height.to_i
    when "blocks"
      options[:command] = :multiple
      options[:height] = height.to_i
    end
  elsif ARGV.length >= 2
    options[:min_height] = ARGV[0].to_i
    options[:max_height] = ARGV[1].to_i
    options[:dir] = ARGV[2] if ARGV[2]
    options[:command] = :range
  else
    puts "Usage: #{$0} MIN_HEIGHT MAX_HEIGHT BLOCKS_DIR"
    puts "   or: #{$0} block HEIGHT STATE_HASH [--dir DIR]"
    puts "   or: #{$0} blocks HEIGHT [--dir DIR]"
    exit 1
  end

  options
end

def list_files_at_url(url)
  uri = URI.parse(url)
  response = Net::HTTP.get_response(uri)

  if response.is_a?(Net::HTTPSuccess)
    # Parse the XML response to extract filenames
    filenames = response.body.scan(/<Key>([^<]+)<\/Key>/).flatten
    filenames.select { |f| f.start_with?("mainnet-") }
  else
    puts "Failed to list files at #{url}: #{response.code} #{response.message}"
    []
  end
end

def download_file(url, target_path)
  if File.exist?(target_path)
    puts "File #{target_path} already exists."
    return true
  end

  temp_path = "#{target_path}.download"

  begin
    uri = URI.parse(url)

    Net::HTTP.start(uri.host, uri.port, use_ssl: uri.scheme == "https") do |http|
      request = Net::HTTP::Get.new(uri)

      http.request(request) do |response|
        if response.is_a?(Net::HTTPSuccess)
          FileUtils.mkdir_p(File.dirname(target_path))

          File.open(temp_path, "wb") do |file|
            response.read_body do |chunk|
              file.write(chunk)
            end
          end

          FileUtils.mv(temp_path, target_path)
          puts "Downloaded #{target_path}. Updating PCB."
          update_pcb(target_path)
        else
          puts "Failed to download #{url}: #{response.code} #{response.message}"
          return false
        end
      end
    end
  rescue => e
    puts "Error downloading #{url}: #{e.message}"
    FileUtils.rm_f(temp_path)
    false
  end
end

def update_pcb(file_path)
  PCB_UPDATER.process_file(file_path)
rescue => e
  puts "Error updating PCB for #{file_path}: #{e.message}"
end

def download_single_block(height, state_hash, dir)
  filename = "mainnet-#{height}-#{state_hash}.json"
  url = "#{BASE_URL}/#{filename}"
  target_path = File.join(dir, filename)

  puts "Attempting to fetch block at height #{height} with state hash #{state_hash}..."
  download_file(url, target_path)
end

def download_all_blocks_at_height(height, dir)
  # List all files at the height
  list_url = URI.parse("#{BASE_URL}?prefix=mainnet-#{height}-")
  puts "Issuing HTTP GET request to #{list_url} to obtain files list."
  response = Net::HTTP.get_response(list_url)

  if response.is_a?(Net::HTTPSuccess)
    filenames = response.body.scan(/<Key>([^<]+)<\/Key>/).flatten
    matching_files = filenames.select { |f| f.start_with?("mainnet-#{height}-") }
  else
    abort "Failed to list blocks at height #{height}: #{response.code} #{response.message}"
  end

  if matching_files.empty?
    puts "No blocks found at height #{height}"
    return
  end

  puts "Found #{matching_files.size} blocks at height #{height}"

  # Sort the matching files
  matching_files.sort!

  # Create a queue of files to download
  matching_files.each do |f|
    url = "#{BASE_URL}/#{f}"
    target_path = File.join(dir, f)

    if download_file(url, target_path)
      puts "Downloaded #{f}"
    else
      abort "Failed to download #{f}"
    end
  end

  puts "Completed downloading blocks at height #{height}"
end

def download_blocks_in_range(min_height, max_height, blocks_dir)
  FileUtils.mkdir_p(blocks_dir)

  puts "Downloading blocks from height #{min_height} to #{max_height} into #{blocks_dir}."

  # Create a queue of block heights to process.
  block_queue = Queue.new
  (min_height..max_height).to_a.each { |height| block_queue << height }

  # Create worker threads to process the block heights queue.
  threads = []
  BLOCK_THREADS.times do
    threads << Thread.new do
      until block_queue.empty?
        begin
          height = block_queue.pop(true)
          download_all_blocks_at_height(height, blocks_dir)
        rescue ThreadError
          # Queue is empty
          break
        end
      end
    end
  end

  # Wait for all block processing to complete
  threads.each(&:join)
  puts "Completed downloading blocks from #{min_height} to #{max_height}"
end

def main_download_mina_blocks
  options = parse_arguments

  case options[:command]
  when :single
    download_single_block(options[:height], options[:state_hash], options[:dir])
  when :multiple
    download_all_blocks_at_height(options[:height], options[:dir])
  when :range
    download_blocks_in_range(options[:min_height], options[:max_height], options[:dir])
  end
end

main_download_mina_blocks if __FILE__ == $0
