#!/usr/bin/env ruby

require "fileutils"
require "optparse"

# Define default parameters
options = {
  source_dir: "/path/to/source",       # Default source directory (change as needed)
  destination_dir: "/path/to/destination", # Default destination directory (change as needed)
  range_min: 1,    # Default minimum range value
  range_max: 25    # Default maximum range value
}

# Parse command-line arguments
OptionParser.new do |opts|
  opts.banner = "Usage: copy_files.rb [options]"

  opts.on("-s", "--source DIR", "Source directory") do |s|
    options[:source_dir] = s
  end

  opts.on("-t", "--target DIR", "Target directory") do |t|
    options[:destination_dir] = t
  end

  opts.on("-r", "--range RANGE", "Range of numbers (e.g., 1-25)") do |r|
    min, max = r.split("-").map(&:to_i)
    options[:range_min] = min
    options[:range_max] = max
  end
end.parse!

# Ensure the destination directory exists
FileUtils.mkdir_p(options[:destination_dir])

# Copy files within the specified range
Dir.glob("#{options[:source_dir]}/*-*.json").each do |file|
  if /-\d+-/.match?(file)  # Check for a number in the filename
    number = file[/-(\d+)-/, 1].to_i

    # Check if the number is within the specified range
    if number.between?(options[:range_min], options[:range_max])
      # Copy the file to the destination directory
      FileUtils.cp(file, options[:destination_dir])
      puts "Copied #{file} to #{options[:destination_dir]}"
    end
  end
end

puts "File copying completed."
