#!/usr/bin/env -S ruby -w

# -*- mode: ruby -*-

require "fileutils"

# Usage: ./move_and_update_blocks.rb <source_folder> <dest_folder> <blocks_list_file> <min_number> <max_number>

# Check if the correct number of arguments is passed
if ARGV.length != 5
  puts "Usage: #{$PROGRAM_NAME} <source_folder> <dest_folder> <blocks_list_file> <min_number> <max_number>"
  exit 1
end

# Assign input parameters to variables
source_folder = ARGV[0]
dest_folder = ARGV[1]
blocks_list_file = ARGV[2]
min_number = ARGV[3].to_i
max_number = ARGV[4].to_i

# Create the destination folder if it doesn't exist
Dir.mkdir(dest_folder) unless Dir.exist?(dest_folder)

# Array to track the moved files
moved_files = []

# Find and move files based on the specified number range
Dir.foreach(source_folder) do |filename|
  next unless /^mainnet-(\d+)-.*\.json$/.match?(filename)
  number = filename.match(/^mainnet-(\d+)-.*\.json$/)[1].to_i

  # Check if the number is within the range
  if number >= min_number && number <= max_number
    source_path = File.join(source_folder, filename)
    dest_path = File.join(dest_folder, filename)

    # Move the file if it exists
    if File.exist?(source_path)
      FileUtils.mv(source_path, dest_path)
      moved_files << filename
      puts "Moved: #{filename} to #{dest_folder}/"
    end
  end
end

# Overwrite the blocks.list with the moved files
File.open(blocks_list_file, "w") do |file|
  moved_files.each { |moved_file| file.puts(moved_file) }
end

puts "Blocks list updated with moved files in #{blocks_list_file}."
