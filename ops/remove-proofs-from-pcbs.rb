#!/usr/bin/env -S ruby -w

# -*- mode: ruby -*-

require "json"
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

# Process a single JSON file
def process_file(path)
  json_data = JSON.parse(File.read(path))

    # Remove all occurrences of "proofs"
  remove_proofs(json_data)

    # Overwrite the original file with compact JSON (single line)
  File.write(path, JSON.generate(json_data))

  puts "Successfully removed 'proofs' from #{path}"
rescue Errno::ENOENT
  puts "File not found: #{path}"
rescue JSON::ParserError
  puts "Invalid JSON file: #{path}"
end

# Process all JSON files in a directory
def process_directory(directory)
  thread_queue = Queue.new
  threads = Array.new(Etc.nprocessors * 4) do
    Thread.new do
      while path = thread_queue.pop
        process_file(path)
      end
    end
  end

  # Feed files to queue as we find them
  Find.find(directory) do |path|
    thread_queue << path if path.end_with?('.json')
  end

  # Signal threads to finish
  threads.size.times { thread_queue << nil }

  # Wait for all threads to complete
  threads.each(&:join)
end

# Usage: ./remove_proofs_from_json_directory_compact.rb <directory>
abort "Usage: #{$PROGRAM_NAME} <directory>" if ARGV.length != 1

process_directory(ARGV[0])
