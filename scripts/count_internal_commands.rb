require "json"

# Function to parse each file and perform aggregation
def parse_file(file_path)
  file_content = File.read(file_path)
  data = JSON.parse(file_content)

  # Initialize a hash to aggregate fees by prover
  aggregated_provers = Hash.new(0)

  # Track any `receiver_pk` from `coinbase[1]` to exclude from aggregation
  excluded_receivers = Set.new

  # Iterate over `staged_ledger_diff.diff` items
  data.dig("staged_ledger_diff", "diff")&.each do |diff|
    next unless diff

    # Check for `coinbase` and exclude `receiver_pk` if present
    coinbase_receiver = diff.dig("coinbase", 1, "receiver_pk")
    if coinbase_receiver
      excluded_receivers << coinbase_receiver
      # Debug: Show excluded receiver
      puts "Excluding receiver_pk: #{coinbase_receiver}"
    end

    # Process `completed_works` entries safely using `dig`
    diff.dig("completed_works")&.each do |work|
      prover = work["prover"]
      fee = work["fee"].to_f

      # Aggregate only non-zero fees, by prover
      if fee > 0
        aggregated_provers[prover] += fee
        # Debug: Show aggregation step
        puts "Aggregating for prover: #{prover}, fee: #{fee}"
      end
    end
  end

  # Filter out excluded provers
  filtered_provers = aggregated_provers.except(*excluded_receivers)

  # Debug: Show filtered provers
  puts "Filtered provers after exclusion: #{filtered_provers.keys}"

  # The final summation is the length of the filtered provers
  filtered_provers.size
rescue JSON::ParserError => e
  puts "Failed to parse JSON in file #{file_path}: #{e.message}"
  0
rescue => e
  puts "An error occurred with file #{file_path}: #{e.message}"
  0
end

# Main function to process directory and calculate summations
def process_directory(directory_path)
  Dir.glob("#{directory_path}/*.json").each do |file_path|
    summation = parse_file(file_path)
    puts "File: #{File.basename(file_path)}, Aggregated Sum: #{summation}"
  end
end

# Run the script with the directory provided as a command-line argument
if ARGV.length != 1
  puts "Usage: ruby script_name.rb <directory_path>"
  exit
end

directory_path = ARGV[0]
process_directory(directory_path)
