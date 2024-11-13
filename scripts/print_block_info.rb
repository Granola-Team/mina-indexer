require "json"

def extract_info(directory)
  Dir.glob("#{directory}/*.json").each do |file_path|
    # Extract height and state_hash from the filename
    if File.basename(file_path) =~ /mainnet-(\d+)-([a-zA-Z0-9]+)\.json$/
      height = $1
      state_hash = $2

      # Parse JSON file contents
      file_content = File.read(file_path)
      json_data = JSON.parse(file_content)

      # Extract last_vrf_output from the JSON data
      last_vrf_output = json_data.dig("protocol_state", "body", "consensus_state", "last_vrf_output")

      # Output the extracted information in a single line
      puts "Height: #{height} | State Hash: #{state_hash} | Last VRF Output: #{last_vrf_output}"
    else
      puts "File #{file_path} does not match the expected naming pattern"
    end
  end
end

# Take directory as an argument
if ARGV.length != 1
  puts "Usage: ruby script.rb <directory>"
else
  extract_info(ARGV[0])
end
