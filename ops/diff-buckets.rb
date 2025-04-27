#!/usr/bin/env -S ruby -w

# Compare the list of historical Mina PCBs in the specified block range between
# O(1)Labs and Granola storage buckets.

require "time"

# Compare the list of historical Mina PCBs in the specified block range between o1Labs and Granola storage buckets.
abort "Usage: #{$0} END_BLOCK" if ARGV[0].nil? || ARGV[0].to_i <= 0

START_BLOCK = 1
END_BLOCK = ARGV[0].to_i
RESULTS_FILE = "diff-buckets.log"
START_TIME = Time.now

def log_time(message)
  elapsed = Time.now - START_TIME
  warn "#{message} (#{elapsed.round(2)} seconds)"
end

class BlockInfo
  attr_reader :height, :state_hash, :filename

  def initialize(filename)
    @filename = filename
    # Strip any folder path to get just the filename
    basename = File.basename(filename)

    # First try the full format (mainnet-HEIGHT-HASH.json)
    match = basename.match(/^mainnet-(\d+)-([A-Za-z0-9]{52,})\.json$/)
    if match && !match[1].empty? && !match[2].empty?
      @height = match[1].to_i
      @state_hash = match[2]
    else
      # Try to match malformed format (mainnet-HASH.json)
      match = basename.match(/^mainnet-([A-Za-z0-9]{52,})\.json$/)
      if match && !match[1].empty?
        @height = nil
        @state_hash = match[1]
      end
    end
  end

  def valid?
    !@state_hash.nil?  # Only require state hash to be valid
  end

  def in_range?(start_block, end_block)
    return true if @height.nil?  # Include blocks with unknown height
    @height >= start_block && @height <= end_block
  end

  def <=>(other)
    return nil unless other.is_a?(BlockInfo)
    return @state_hash <=> other.state_hash if @height.nil? || other.height.nil?
    [@height, @state_hash] <=> [other.height, other.state_hash]
  end

  def eql?(other)
    self.class == other.class && @state_hash == other.state_hash
  end

  def hash
    @state_hash.hash
  end

  def to_s
    if @height
      "Block #{@height} - #{@state_hash}"
    else
      "Block (unknown height) - #{@state_hash}"
    end
  end
end

def read_existing_list(filename)
  return [] unless File.exist?(filename) && !File.empty?(filename)
  warn "Reading existing file #{filename}"
  lines = File.readlines(filename, chomp: true)
  warn "Read #{lines.size} lines from #{filename}"
  lines
end

def write_list_file(filename, list)
  warn "Writing #{list.size} entries to #{filename}"
  return if list.empty?
  File.write(filename, list.join("\n"))
end

def fetch_and_sort_blocks(source, cmd)
  warn "Creating block list for #{source}, issuing: #{cmd}"
  contents = `#{cmd}`

  if contents.nil? || contents.empty?
    warn "ERROR: Empty or nil response from command: #{cmd}"
    return []
  end

  warn "Received #{contents.bytesize} bytes from #{source}"

  initial_list = contents.lines(chomp: true)
  warn "Initial line count for #{source}: #{initial_list.size}"

  initial_list.select! { |f| f.start_with?("mainnet-") }

  blocks = initial_list.map { |f| BlockInfo.new(f) }
  valid_blocks = blocks.select(&:valid?)
  in_range_blocks = valid_blocks.select { |b| b.in_range?(START_BLOCK, END_BLOCK) }

  result = in_range_blocks.sort.map(&:filename)
  warn "Final result count for #{source}: #{result.size}"
  result
end

def find_malformed_matches(granola_blocks, o1_list)
  # Create a map of state hash -> original filename for o1 files
  o1_malformed = o1_list.map { |f| BlockInfo.new(f) }.select { |b| b.valid? }
  o1_hash_map = o1_malformed.each_with_object({}) { |b, h| h[b.state_hash] = b }

  matches = []
  granola_blocks.each do |granola_block|
    if (o1_match = o1_hash_map[granola_block.state_hash])
      matches << {
        granola: granola_block,
        o1: o1_match
      }
    end
  end
  matches
end

# Read existing files if they exist
existing_o1 = read_existing_list("o1.list")
existing_granola = read_existing_list("granola.list")

if !existing_o1.empty? && !existing_granola.empty?
  warn "Using existing files"
  o1_list = existing_o1
  granola_list = existing_granola
else
  warn "Performing fresh downloads..."

  o1_thread = Thread.new do
    o1_cmd = "rclone --config #{__dir__}/rclone.conf lsf o1:mina_network_block_data"
    fetch_and_sort_blocks("o1Labs", o1_cmd)
  end

  granola_thread = Thread.new do
    granola_cmd = "#{__dir__}/granola-rclone.rb lsf linode-granola:blocks.minasearch.com"
    fetch_and_sort_blocks("Granola", granola_cmd)
  end

  warn "Waiting for threads to complete..."
  o1_list = o1_thread.value
  granola_list = granola_thread.value

  write_list_file("o1.list", o1_list)
  write_list_file("granola.list", granola_list)
end

log_time("Starting comparison")

# Convert lists to BlockInfo objects
o1_blocks = o1_list.map { |f| BlockInfo.new(f) }
granola_blocks = granola_list.map { |f| BlockInfo.new(f) }

# Find invalid filenames
invalid_o1 = o1_list.select { |f| !BlockInfo.new(f).valid? }
invalid_granola = granola_list.select { |f| !BlockInfo.new(f).valid? }

# Remove invalid entries
o1_blocks.select!(&:valid?)
granola_blocks.select!(&:valid?)

# Create sets for comparison based on state hash
o1_set = Set.new(o1_blocks)
granola_set = Set.new(granola_blocks)

# Find unique blocks in each source
blocks_only_in_o1 = o1_set - granola_set
blocks_only_in_granola = granola_set - o1_set

# Check for malformed matches
if !blocks_only_in_granola.empty?
  warn "\nChecking Granola 'unique' blocks for matches in o1..."
  matches = find_malformed_matches(blocks_only_in_granola.to_a, o1_list)

  if matches.any?
    warn "Found #{matches.size} blocks with matching state hashes:"
    matches.each do |match|
      warn "  Match: #{match[:granola].filename} -> #{match[:o1].filename}"
    end

    # Remove matched blocks from the unique list
    matched_blocks = Set.new(matches.map { |m| m[:granola] })
    blocks_only_in_granola -= matched_blocks
  end
end

log_time("Comparison completed")

# Write detailed results to file
File.open(RESULTS_FILE, "w") do |f|
  f.puts "Comparison Results (blocks #{START_BLOCK} to #{END_BLOCK})"
  f.puts "Run started at: #{START_TIME}"
  f.puts "Run completed at: #{Time.now}"
  f.puts "=" * 50

  if !invalid_o1.empty? || !invalid_granola.empty?
    f.puts "\nInvalid filenames found:"
    f.puts "-" * 30
    f.puts "o1Labs: #{invalid_o1.size}"
    f.puts "Granola: #{invalid_granola.size}"
  end

  f.puts "\nUnique blocks in o1Labs (#{blocks_only_in_o1.size}):"
  f.puts "-" * 30
  blocks_only_in_o1.sort.each { |block| f.puts block }

  f.puts "\nUnique blocks in Granola (#{blocks_only_in_granola.size}):"
  f.puts "-" * 30
  blocks_only_in_granola.sort.each { |block| f.puts block }
end

# Print summary to screen
puts "\nComparison Summary:"
puts "=" * 20
puts "Total valid blocks in o1Labs: #{o1_blocks.size}"
puts "Total valid blocks in Granola: #{granola_blocks.size}"
puts "Invalid filenames in o1Labs: #{invalid_o1.size}"
puts "Invalid filenames in Granola: #{invalid_granola.size}"
puts "Unique blocks in o1Labs: #{blocks_only_in_o1.size}"
puts "Unique blocks in Granola: #{blocks_only_in_granola.size}"
puts "\nTotal runtime: #{(Time.now - START_TIME).round(2)} seconds"
puts "Detailed results written to: #{RESULTS_FILE}"
