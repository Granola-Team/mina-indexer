#!/usr/bin/env -S ruby -w

# Compare the list of historical Mina PCBs in the specified block range between o1Labs and Granola
START_BLOCK = 1
END_BLOCK = ARGV[0]
RESULTS_FILE = "diff-buckets.log"

def parse_block_number(filename)
  # Match pattern: mainnet-NUMBER-HASH.json
  match = filename.match(/^mainnet-(\d+)-[A-Za-z0-9]+\.json$/)
  return nil unless match && !match[1].empty? # Ensure we have a valid number
  match[1].to_i
end

def read_existing_list(filename)
  return [] unless File.exist?(filename)
  File.readlines(filename, chomp: true)
end

def fetch_and_sort_blocks(source, cmd, filter_prefix: nil)
  warn "Creating block list for #{source}, issuing: #{cmd}"
  contents = `#{cmd}` || abort("Failure: #{cmd}")

  # Get initial list and apply prefix filter if specified
  initial_list = contents.lines(chomp: true)
  initial_list.select! { |f| f.start_with?(filter_prefix) } if filter_prefix

  # Only keep files matching the pattern mainnet-NUMBER-HASH.json
  initial_list.select! { |f| f.match?(/^mainnet-\d+-[A-Za-z0-9]+\.json$/) }

  # Sort the list
  sorted = initial_list.sort! do |a, b|
    a_num = parse_block_number(a)
    b_num = parse_block_number(b)

    # Both should be valid numbers at this point
    a_num <=> b_num
  end

  # Filter based on block range
  sorted
    .select { |f|
      num = parse_block_number(f)
      num && num >= START_BLOCK && num <= END_BLOCK.to_i
    }
end

# Read existing files if they exist
existing_o1 = read_existing_list("o1.list")
existing_granola = read_existing_list("granola.list")

if !existing_o1.empty? && !existing_granola.empty?
  warn "Using existing files"
  o1_list = existing_o1
  granola_list = existing_granola
else
  # Create threads for parallel downloads
  o1_thread = Thread.new do
    o1_cmd = "#{__dir__}/granola-rclone.rb lsf o1:mina_network_block_data"
    fetch_and_sort_blocks("o1Labs", o1_cmd, filter_prefix: "mainnet-")
  end

  granola_thread = Thread.new do
    granola_cmd = "#{__dir__}/granola-rclone.rb lsf cloudflare:mina-blocks"
    fetch_and_sort_blocks("Granola", granola_cmd, filter_prefix: "mainnet-")
  end

  # Wait for both threads to complete and get their results
  o1_list = o1_thread.value
  granola_list = granola_thread.value

  # Write results to files
  File.write("o1.list", o1_list.join("\n"))
  File.write("granola.list", granola_list.join("\n"))
end

# Find invalid filenames
invalid_o1 = o1_list.select { |f| parse_block_number(f).nil? }
invalid_granola = granola_list.select { |f| parse_block_number(f).nil? }

# Remove invalid entries before comparison
o1_list -= invalid_o1
granola_list -= invalid_granola

# Create hash sets of block numbers for comparison
o1_blocks = Set.new(o1_list.map { |f| parse_block_number(f) })
granola_blocks = Set.new(granola_list.map { |f| parse_block_number(f) })

# Find missing blocks in each source
blocks_only_in_o1 = o1_blocks - granola_blocks
blocks_only_in_granola = granola_blocks - o1_blocks

# Get the corresponding files for missing blocks
files_only_in_o1 = o1_list.select { |f| blocks_only_in_o1.include?(parse_block_number(f)) }.sort
files_only_in_granola = granola_list.select { |f| blocks_only_in_granola.include?(parse_block_number(f)) }.sort

# Write detailed results to file
File.open(RESULTS_FILE, "w") do |f|
  f.puts "Comparison Results (blocks #{START_BLOCK} to #{END_BLOCK})"
  f.puts "=" * 50

  if !invalid_o1.empty? || !invalid_granola.empty?
    f.puts "\nInvalid filenames found:"
    f.puts "-" * 30
    f.puts "\nIn o1Labs (#{invalid_o1.size}):"
    invalid_o1.each { |file| f.puts file }
    f.puts "\nIn Granola (#{invalid_granola.size}):"
    invalid_granola.each { |file| f.puts file }
  end

  f.puts "\nBlocks missing from Granola (#{blocks_only_in_o1.size}):"
  f.puts "-" * 30
  blocks_only_in_o1.to_a.sort.each { |block| f.puts "Block #{block}" }
  f.puts "\nCorresponding files in o1Labs:"
  files_only_in_o1.each { |file| f.puts file }

  f.puts "\nBlocks missing from o1Labs (#{blocks_only_in_granola.size}):"
  f.puts "-" * 30
  blocks_only_in_granola.to_a.sort.each { |block| f.puts "Block #{block}" }
  f.puts "\nCorresponding files in Granola:"
  files_only_in_granola.each { |file| f.puts file }
end

# Print summary to screen
puts "\nComparison Summary:"
puts "=" * 20
puts "Total valid blocks in o1Labs: #{o1_blocks.size}"
puts "Total valid blocks in Granola: #{granola_blocks.size}"
puts "Invalid filenames in o1Labs: #{invalid_o1.size}"
puts "Invalid filenames in Granola: #{invalid_granola.size}"
puts "Blocks only in o1Labs: #{blocks_only_in_o1.size}"
puts "Blocks only in Granola: #{blocks_only_in_granola.size}"
puts "\nDetailed results written to: #{RESULTS_FILE}"
