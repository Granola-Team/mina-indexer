#!/usr/bin/env -S ruby -w

# -*- mode: ruby -*-

require 'set'

BLOCK_PATTERN = /mainnet-(\d+)-/.freeze

def check_sequence(directory_path)
  # Use Set for O(1) lookups
  block_numbers = Set.new

  # First pass - collect all block numbers
  Dir.each_child(directory_path) do |filename|
    next unless filename.end_with?('.json')
    if match = BLOCK_PATTERN.match(filename)
      block_numbers.add(match[1].to_i)
    end
  end

  min_block = block_numbers.min
  max_block = block_numbers.max

  puts "\nBlock Statistics:"
  puts "Total unique block heights: #{block_numbers.size}"
  puts "Range: #{min_block} to #{max_block}"

  # Find gaps - much simpler with a Set
  missing_blocks = (min_block..max_block).to_set - block_numbers

  if missing_blocks.empty?
    puts "\n✅ Sequence is contiguous!"
  else
    puts "\n❌ Found gaps in sequence:"
    # Convert missing blocks to ranges for cleaner output
    ranges = missing_blocks.to_a.sort.chunk_while { |i, j| i + 1 == j }.to_a
    ranges.each do |range|
      if range.size == 1
        puts "Missing block: #{range.first}"
      else
        puts "Missing blocks: #{range.first} to #{range.last}"
      end
    end
  end
end

def print_usage
  puts "Usage: #{$PROGRAM_NAME} <directory_path>"
  puts
  puts "Arguments:"
  puts "  directory_path    Path to directory containing Mina PCB JSON files"
  puts
  puts "Example:"
  puts "  #{$PROGRAM_NAME} /path/to/blocks/directory"
  puts
  puts "Description:"
  puts "  Scans a directory of Mina PCB JSON files and checks for"
  puts "  contiguous block numbers in the filenames."
  exit 1
end

# Main execution
begin
  if ARGV.empty? || ARGV[0] == '-h' || ARGV[0] == '--help'
    print_usage
  end

  directory_path = ARGV[0]
  raise ArgumentError, "Directory not found: #{directory_path}" unless Dir.exist?(directory_path)

  start_time = Process.clock_gettime(Process::CLOCK_MONOTONIC)
  check_sequence(directory_path)
  end_time = Process.clock_gettime(Process::CLOCK_MONOTONIC)

  puts "\nTotal execution time: #{(end_time - start_time).round(2)} seconds"
rescue => e
  puts "Error: #{e.message}"
  print_usage
end
