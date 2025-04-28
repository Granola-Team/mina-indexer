#!/usr/bin/env ruby

require "json"
require "fileutils"
require "open3"
require "etc"

# Remove both `proofs` and `protocol_state_proof` and add a v2 hash for any transaction.
class PcbUpdater
  MINA_TXN_HASHER = "mina_txn_hasher.exe"

  # Add transaction hashes starting from this block height
  V2_BLOCKCHAIN_START = 359605

  def initialize
    @queue = Queue.new
    @num_workers = Etc.nprocessors * 4
  end

  def process_files(output_dir)
    # Create worker threads
    threads = []
    @num_workers.times do
      threads << Thread.new do
        until @queue.empty?
          path = begin
            @queue.pop(true)
          rescue
            nil
          end
          break unless path
          process_file(path, output_dir)
        end
      end
    end

    puts "Processing with #{@num_workers} worker threads"

    # Wait for all threads to complete
    threads.each(&:join)
  end

  # Add a file to the processing queue
  def add_file(path)
    @queue << path if File.file?(path) && path.end_with?(".json")
  end

  def add_directory(dir)
    Dir.foreach(dir) do |entry|
      add_file(File.join(dir, entry))
    end
  end

  # Process a single JSON file to remove `proofs` and `protocol_state_proof` and add v2 transaction hashes
  def process_file(path, output_dir)
    json_data = JSON.parse(File.read(path))

    remove_proofs(json_data)
    json_data.delete("protocol_state_proof")
    json_data["data"]&.delete("protocol_state_proof")

    # Check blockchain length before adding transaction hashes
    blockchain_length = get_blockchain_length(json_data)

    if blockchain_length.to_i >= V2_BLOCKCHAIN_START
      add_transaction_hashes(json_data, File.basename(path))
    end

    # Write the modified JSON in compact format.
    outfile = File.join(output_dir, File.basename(path))
    File.write(outfile, JSON.generate(json_data))
    puts "Wrote #{outfile}"
  rescue => e
    abort "Error processing #{path}: #{e.message}"
  end

  def get_blockchain_length(json_data)
    json_data.dig("data", "protocol_state", "body", "consensus_state", "blockchain_length")&.to_i ||
      raise("Error extracting blockchain length")
  end

  # Recursively remove proofs from the JSON
  def remove_proofs(obj)
    case obj
    when Hash
      # Remove proof fields
      obj.delete("proofs")

      # Process remaining fields
      obj.each_value { |v| remove_proofs(v) }
    when Array
      obj.each { |v| remove_proofs(v) }
    end
  end

  # Find all transactions and add hashes
  def add_transaction_hashes(json_data, filename)
    # Find all transactions
    transactions = find_transactions(json_data)

    if transactions.empty?
      return
    end

    # Process all transactions and update the JSON
    transactions.each do |command_obj, txn_data|
      # Skip if already has a hash
      next if command_obj.key?("txn_hash")

      hash = compute_hash(txn_data, filename)
      command_obj["txn_hash"] = hash if hash
    end
  end

  # Find all transactions and return a hash of {command_obj => txn_data}
  def find_transactions(json_data)
    transactions = {}
    queue = [[json_data, []]]

    until queue.empty?
      obj, path = queue.shift

      case obj
      when Hash
        if obj["data"]&.is_a?(Array) &&
            (obj["data"][0] == "Signed_command" || obj["data"][0] == "Zkapp_command")
          # Found a transaction (either signed_command or zkapp_command)
          transactions[obj] = obj["data"][1]
        else
          # Continue searching in all values
          obj.each do |key, value|
            queue << [value, path + [key]]
          end
        end
      when Array
        # Continue searching in all array elements
        obj.each_with_index do |item, index|
          queue << [item, path + [index]]
        end
      end
    end

    transactions
  end

  # Compute hash for a transaction
  def compute_hash(txn_data, filename)
    cmd = "#{MINA_TXN_HASHER} '#{JSON.generate(txn_data)}'"
    stdout, stderr, status = Open3.capture3(cmd)

    if status.success?
      stdout.strip
    else
      # Report filename when hasher fails
      puts "Error running hasher: #{stderr}"
      puts "File: #{filename}"
      nil
    end
  end
end

def main
  if ARGV.size != 2
    puts "Usage: #{$0} [FILE_OR_DIRECTORY] [OUTPUT_DIR]"
    exit 1
  end

  processor = PcbUpdater.new

  in_path = ARGV[0]
  output_dir = ARGV[1]
  in_path.each do |path|
    if File.directory?(path)
      processor.add_directory(path)
    else
      processor.add_file(path)
    end
  end

  processor.process_files(output_dir)
end

# Only execute main when this script is run directly, not when required
main if __FILE__ == $0
