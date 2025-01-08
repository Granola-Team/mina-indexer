#!/usr/bin/env ruby

require "json"
require "optparse"
require "pathname"

################################################################################
# Command-line options
################################################################################

options = {
  root_file: nil,
  source_dir: ".",
  fork: false
}

OptionParser.new do |opts|
  opts.banner = "Usage: purge_branch.rb [options]"

  opts.on("-r", "--root FILE", "Root file (lowest block in the chain)") do |r|
    options[:root_file] = r
  end

  opts.on("-s", "--source DIR", "Source directory (defaults to current)") do |s|
    options[:source_dir] = s
  end

  opts.on("-f", "--fork", "Assume forked JSON structure (parse from data.protocol_state)") do
    options[:fork] = true
  end
end.parse!

unless options[:root_file]
  puts "You must provide --root <file> and source dir --source-dir <path/to/folder>"
  exit 1
end

################################################################################
# Utility: parse the file’s height/hash from its name
################################################################################

def parse_filename(file_path)
  # Expects something like: mainnet-<height>-<hash>.json
  basename = File.basename(file_path)
  if m = basename.match(/-(\d+)-([a-zA-Z0-9]+)\.json$/)
    height = m[1].to_i
    block_hash = m[2]
    return [height, block_hash]
  end
  nil
end

################################################################################
# Utility: read previous_state_hash from JSON
################################################################################

def get_previous_state_hash(file_path, is_fork)
  content = JSON.parse(File.read(file_path))
  if is_fork
    content.dig("data", "protocol_state", "previous_state_hash")
  else
    content.dig("protocol_state", "previous_state_hash")
  end
end

################################################################################
# Find all children of the given block (i.e., files that reference this block's hash)
################################################################################

def find_children(source_dir, parent_height, parent_hash, is_fork)
  # A child must have height == parent_height + 1
  # And must have previous_state_hash == parent_hash
  target_height = parent_height + 1

  # Build a glob to match: mainnet-(target_height)-*.json
  pattern = File.join(source_dir, "mainnet-#{target_height}-*.json")

  # For each candidate, parse JSON, compare previous_state_hash
  Dir.glob(pattern).each_with_object([]) do |child_path, arr|
    if parse_filename(child_path)
      # parse_filename => [child_height, child_hash]
      child_height, child_hash = parse_filename(child_path)
      # read previous_state_hash
      prev = get_previous_state_hash(child_path, is_fork)
      # if prev == parent_hash => it's a child
      if prev == parent_hash
        arr << child_path
      end
    end
  end
end

################################################################################
# Recursive DFS to gather all descendants
################################################################################

def gather_descendants(file_path, is_fork, visited, results, source_dir)
  return if visited.include?(file_path)
  visited << file_path
  results << file_path

  info = parse_filename(file_path)
  return unless info
  parent_height, parent_hash = info

  # find children => for each => recurse
  children = find_children(source_dir, parent_height, parent_hash, is_fork)
  children.each do |child_file|
    gather_descendants(child_file, is_fork, visited, results, source_dir)
  end
end

################################################################################
# Main
################################################################################

root_file = File.join(options[:source_dir], options[:root_file])
unless File.exist?(root_file)
  puts "Root file not found: #{root_file}"
  exit 1
end

visited = Set.new
results = []

gather_descendants(root_file, options[:fork], visited, results, options[:source_dir])

# puts "Found #{results.size} block file(s) connected to root."
# puts "These files can be removed with:"
results.each do |f|
  puts "rm \"#{f}\""
end
