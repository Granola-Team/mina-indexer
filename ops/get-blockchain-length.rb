def get_blockchain_length(json_data)
  len = json_data.dig("protocol_state", "body", "consensus_state", "blockchain_length")
  if len
    len
  else
    len = json_data.dig("data", "protocol_state", "body", "consensus_state", "blockchain_length")
    len || abort("Error extracting blockchain length")
  end
end
