#!/usr/bin/env ruby

require "json"
require "find"

# Recursive method to remove all "proofs" properties from a JSON structure
def remove_proofs(obj)
  if obj.is_a?(Hash)
    obj.delete("proofs")
    obj.each { |key, value| remove_proofs(value) }
  elsif obj.is_a?(Array)
    obj.each { |item| remove_proofs(item) }
  end
end

# Process all JSON files in a directory
def process_directory(directory, processed_files)
  Find.find(directory) do |path|
    if /\.json$/.match?(path) && !processed_files.include?(path)
      begin
        file_contents = File.read(path)
        json_data = JSON.parse(file_contents)

        # Remove all occurrences of "proofs"
        remove_proofs(json_data)

        # Overwrite the original file with compact JSON (single line)
        File.write(path, JSON.generate(json_data))

        # Add file to processed list
        processed_files << path

        puts "Successfully removed 'proofs' from #{path}"
      rescue Errno::ENOENT
        puts "File not found: #{path}"
      rescue JSON::ParserError
        puts "Invalid JSON file: #{path}"
      end
    end
  end
end

# Usage: ./remove_proofs_from_pcbs.rb <directory>
if ARGV.length != 1
  puts "Usage: #{$PROGRAM_NAME} <directory>"
  exit 1
end

directory = ARGV[0]
processed_files = [] # Array to store the processed files

# Loop indefinitely
loop do
  process_directory(directory, processed_files)
  puts "Waiting for new files to process..."

  # Sleep for 10 minutes before the next iteration (600 seconds)
  sleep 600
end
