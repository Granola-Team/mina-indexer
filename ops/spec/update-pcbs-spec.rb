require "json"
require_relative "../update-pcbs"

RSpec.describe PcbUpdater do
  let(:test_directory) { "spec/test_json" }
  let(:test_file) { "#{test_directory}/test.json" }
  let(:output_dir) { "spec/out" }
  let(:out_file) { "#{output_dir}/test.json" }
  let(:updater) { PcbUpdater.new }

  # JSON content with "proofs" property and blockchain data
  let(:json_content) do
    {
      "data" => {
        "id" => 1,
        "name" => "example",
        "proofs" => {
          "signature" => "123abc"
        },
        "protocol_state" => {
          "body" => {
            "consensus_state" => {
              "blockchain_length" => 360000  # Above V2_BLOCKCHAIN_START
            }
          }
        },
        "details" => {
          "info" => "some data",
          "proofs" => "another proof"
        },
        "data" => [
          "Signed_command",
          {
            payload: {
              common: {
                fee: "0.1",
                nonce: 5
              },
              body: [
                "Payment",
                {
                  source_pk: "B62...",
                  receiver_pk: "B62...",
                  amount: "50"
                }
              ]
            }
          }
        ]
      }
    }
  end

  # Expected JSON content after processing
  let(:expected_content_structure) do
    {
      "data" => {
        "id" => 1,
        "name" => "example",
        "protocol_state" => {
          "body" => {
            "consensus_state" => {
              "blockchain_length" => 360000
            }
          }
        },
        "details" => {
          "info" => "some data"
        },
        "data" => [
          "Signed_command",
          {
            payload: {
              common: {
                fee: "0.1",
                nonce: 5
              },
              body: [
                "Payment",
                {
                  source_pk: "B62...",
                  receiver_pk: "B62...",
                  amount: "50"
                }
              ]
            }
          }
        ]
      }
    }
  end

  before do
    # Create the spec directory if it doesn't exist
    Dir.mkdir("spec") unless Dir.exist?("spec")

    # Create test directory and file
    Dir.mkdir(test_directory) unless Dir.exist?(test_directory)
    Dir.mkdir(output_dir) unless Dir.exist?(output_dir)

    # Write the JSON content with "proofs" property to test file
    File.write(test_file, JSON.generate(json_content))

    # Mock the compute_hash method to return a predictable hash
    allow_any_instance_of(PcbUpdater).to receive(:compute_hash).and_return("mock_transaction_hash")
  end

  after do
    # Clean up - remove the test directory and files
    File.delete(out_file) if File.exist?(test_file)
    File.delete(test_file) if File.exist?(test_file)
    Dir.rmdir(output_dir) if Dir.exist?(output_dir)
    Dir.rmdir(test_directory) if Dir.exist?(test_directory)
    Dir.rmdir("spec") if Dir.exist?("spec") && Dir.empty?("spec")
  end

  it 'removes all occurrences of "proofs" and keeps JSON compact' do
    # Add the test file to the updater
    updater.add_file(test_file)

    # Process the file
    updater.process_files(output_dir)

    # Read the processed file
    processed_content = File.read(out_file)
    processed_json = JSON.parse(processed_content)

    # Check if the "proofs" property is removed
    expect(processed_json.dig("data", "proofs")).to be_nil
    expect(processed_json.dig("data", "details", "proofs")).to be_nil

    # Check that the JSON remains compact (no new lines or pretty printing)
    expect(processed_content).not_to include("\n")
  end

  it "adds transaction hashes for blocks above the V2 threshold" do
    # Add the test file to the updater
    updater.add_file(test_file)

    # Process the file
    updater.process_files(output_dir)

    # Read the processed file
    processed_json = JSON.parse(File.read(out_file))

    # Check if transaction hash was added
    expect(processed_json.dig("data", "txn_hash")).to eq("mock_transaction_hash")
  end

  it "correctly processes a directory of files" do
    # Add the test directory to the updater
    updater.add_directory(test_directory)

    # Process all files
    updater.process_files(output_dir)

    # Read the processed file
    processed_json = JSON.parse(File.read(out_file))

    # Check basic structure is maintained
    expect(processed_json.keys).to eq(expected_content_structure.keys)
  end
end
