#!/usr/bin/env -S ruby -w

# -*- mode: ruby -*-

require "json"
require "fileutils"
require "find"
require "etc"

# Recursive method to remove all "proofs" properties from a JSON structure
def remove_proofs(obj)
  case obj
  when Hash
    obj.delete("proofs")
    obj.each_value { |value| remove_proofs(value) }
  when Array
    obj.each { |item| remove_proofs(item) }
  end
end

# Process a single JSON file to remove proofs
def process_json_file(path)
  json_data = JSON.parse(File.read(path))

  # Remove all occurrences of "proofs"
  remove_proofs(json_data)

  # Overwrite the original file with compact JSON (single line)
  File.write(path, JSON.generate(json_data))

  if __FILE__ == $0
    puts "Successfully removed 'proofs' from #{path}"
  end
rescue JSON::ParserError
  puts "Invalid JSON file: #{path}"
rescue => e
  puts "Error processing JSON file #{path}: #{e.message}"
end

# Process all JSON files in a directory (and its subdirectories)
def process_directory(dir)
  puts "Processing JSON files in #{dir}..."

  # Find all JSON files in the directory and its subdirectories
  json_files = []
  Find.find(dir) do |path|
    if FileTest.file?(path) && path.end_with?(".json")
      json_files << path
    end
  end

  if json_files.empty?
    puts "No JSON files found in #{dir}"
    return
  end

  puts "Found #{json_files.size} JSON files"

  # Determine the number of worker threads based on CPU count
  num_workers = Etc.nprocessors * 4
  puts "Using #{num_workers} worker threads"

  # Create a thread-safe queue for files to process
  queue = Queue.new
  json_files.each { |file| queue << file }

  # Create worker threads to process files in parallel
  threads = []
  num_workers.times do
    threads << Thread.new do
      until queue.empty?
        begin
          file = queue.pop(true)
          process_json_file(file)
        rescue ThreadError
          # Queue is empty
          break
        end
      end
    end
  end

  # Wait for all threads to complete
  threads.each(&:join)

  puts "Finished processing all JSON files"
end

def main_remove_proofs_from_pcbs
  if ARGV.empty?
    puts "Usage: #{$0} DIRECTORY_OR_FILE [DIRECTORY_OR_FILE ...]"
    exit 1
  end

  ARGV.each do |path|
    if File.directory?(path)
      process_directory(path)
    elsif File.file?(path) && path.end_with?(".json")
      process_json_file(path)
    else
      puts "Skipping #{path}: Not a JSON file or directory"
    end
  end
end

# Only execute main when this script is run directly, not when required
main_remove_proofs_from_pcbs if __FILE__ == $0
