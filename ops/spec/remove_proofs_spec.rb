require 'json'
require_relative '../remove-proofs-from-pcbs'

RSpec.describe 'Remove Proofs Script' do
  let(:test_directory) { 'spec/test_json' }
  let(:test_file) { "#{test_directory}/test.json" }

  # JSON content with "proofs" property
  let(:json_content) do
    {
      "data" => {
        "id" => 1,
        "name" => "example",
        "proofs" => {
          "signature" => "123abc"
        },
        "details" => {
          "info" => "some data",
          "proofs" => "another proof"
        }
      }
    }
  end

  # Expected JSON content after removing "proofs"
  let(:expected_content) do
    {
      "data" => {
        "id" => 1,
        "name" => "example",
        "details" => {
          "info" => "some data"
        }
      }
    }
  end

  before do
    # Create the spec directory if it doesn't exist
    Dir.mkdir('spec') unless Dir.exist?('spec')

    # Create test directory and file
    Dir.mkdir(test_directory) unless Dir.exist?(test_directory)

    # Write the JSON content with "proofs" property to test file
    File.open(test_file, 'w') { |f| f.write(JSON.generate(json_content)) }
  end

  after do
    # Clean up - remove the test directory and files
    File.delete(test_file) if File.exist?(test_file)
    Dir.rmdir(test_directory) if Dir.exist?(test_directory)
    Dir.rmdir('spec') if Dir.exist?('spec') && Dir.empty?('spec')
  end

  it 'removes all occurrences of "proofs" and keeps JSON compact' do
    # Run the script on the test directory
    process_directory(test_directory)

    # Read the processed file
    processed_content = File.read(test_file)
    processed_json = JSON.parse(processed_content)

    # Check if the "proofs" property is removed and the rest of the data remains intact
    expect(processed_json).to eq(expected_content)

    # Check that the JSON remains compact (no new lines or pretty printing)
    expect(processed_content).not_to include("\n")
  end
end
