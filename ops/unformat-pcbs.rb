#!/usr/bin/env -S ruby -w

# -*- mode: ruby -*-

require "json"
require "find"
require "etc"

# Process a single JSON file by compacting it
def process_file(path)
  json_data = JSON.parse(File.read(path))

  # Write the compact JSON (single line)
  File.write(path, JSON.generate(json_data))

  puts "Processed: #{path}"
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
      while (path = thread_queue.pop)
        process_file(path)
      end
    end
  end

  # Feed files to queue as we find them
  Find.find(directory) do |path|
    thread_queue << path if path.end_with?(".json")
  end

  # Signal threads to finish
  threads.size.times { thread_queue << nil }

  # Wait for all threads to complete
  threads.each(&:join)
end

# Usage: ./unformat-pcbs.rb <directory>
abort "Usage: #{$PROGRAM_NAME} <directory>" if ARGV.length != 1

process_directory(ARGV[0])
