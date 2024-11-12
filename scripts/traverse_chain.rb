#!/usr/bin/env ruby

require 'json'
require 'optparse'

# Define default options
options = {
  source_dir: '/path/to/source',
  tip_file: 'mainnet-25-<tip_hash>.json'
}

# Parse command-line options
OptionParser.new do |opts|
  opts.banner = "Usage: traverse_chain.rb [options]"

  opts.on("-s", "--source DIR", "Source directory") do |s|
    options[:source_dir] = s
  end

  opts.on("-t", "--tip FILE", "Tip file (starting file in the chain)") do |t|
    options[:tip_file] = t
  end
end.parse!

# Helper function to get the previous state hash from file contents
def get_previous_state_hash(file_path)
  file_content = JSON.parse(File.read(file_path))
  file_content.dig("protocol_state", "previous_state_hash")
end

# Traverse the blockchain from the tip to the root
def traverse_chain(source_dir, tip_file)
  current_file = File.join(source_dir, tip_file)
  chain = []

  puts "current file: #{current_file}"

  while File.exist?(current_file)
    # Extract height and current hash from filename
    height, current_hash = current_file.match(/-(\d+)-([a-zA-Z0-9]+)\.json$/).captures
    height = height.to_i

    # Append the current file to the chain
    chain << current_file
    puts "Traversed #{File.basename(current_file)} (Height: #{height}, Hash: #{current_hash})"

    # Stop if we've reached the root (height 0)
    break if height == 0

    # Get the previous state hash to find the next file in the chain
    prev_state_hash = get_previous_state_hash(current_file)

    # Decrement the height and find the next file
    next_file = "#{source_dir}/mainnet-#{height - 1}-#{prev_state_hash}.json"
    puts "next file: #{next_file}"
    current_file = next_file
  end

  puts "\nTraversal completed. Chain from tip to root:"
  chain.each { |file| puts File.basename(file) }
end

# Start the traversal from the tip file
traverse_chain(options[:source_dir], options[:tip_file])
