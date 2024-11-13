require "json"
require "pathname"

def sum_array_length_in_files(directory_path)
  total_count = 0

  # Iterate over each JSON file in the directory
  Dir.glob(File.join(directory_path, "*.json")).each do |file_path|
    file_content = File.read(file_path)
    json_data = JSON.parse(file_content)

    # Access the arrays and handle potential null values
    diff = json_data.dig("staged_ledger_diff", "diff")
    next if diff.nil? # Skip if diff is not present

    # Add lengths of commands in diff[0] and diff[1] if they exist
    total_count += diff[0]&.dig("commands")&.length.to_i
    total_count += diff[1]&.dig("commands")&.length.to_i
  end

  total_count
end

# Get directory path from the command line argument
directory_path = ARGV[0]

if directory_path.nil? || !Dir.exist?(directory_path)
  puts "Please provide a valid directory path."
else
  result = sum_array_length_in_files(directory_path)
  puts "Total number of commands across all files: #{result}"
end
