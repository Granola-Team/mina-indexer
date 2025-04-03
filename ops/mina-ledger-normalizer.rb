#!/usr/bin/env -S ruby -w

require "json"

MINA_TOKEN = "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf"

filename = ARGV[0]
file = File.read(filename)
accounts = JSON.parse(file)

# Handle both ledger styles
accounts = accounts["ledger"]["accounts"] if !accounts.instance_of?(Array)

def normalize_zkapp(data)
  return data if data.nil?

  result = data

  # convert integers to hex strings
  result["app_state"] = data["app_state"].map { |app| normalize_hex(app) }
  result["action_state"] = data["action_state"].map { |action| normalize_hex(action) }

  result
end

def normalize_hex(data)
  hex_str = data.to_i.to_s(16)
  "0x" + hex_str.ljust(64, "0")
end

result = {}

# Normalize all token accounts
accounts.each do |account|
  pk = account["pk"]
  raise "Missing public key: #{JSON.pretty_generate(account)}" if pk.nil?

  token = if account["token"].nil?
    MINA_TOKEN
  elsif account["token"] == "1"
    # The MINA token was, pre-hardfork, token 1.
    MINA_TOKEN
  else
    account["token"]
  end

  result[token] ||= {}
  result[token][pk] =
    {
      "nonce" => account["nonce"] || "0",
      "balance" => account["balance"],
      "delegate" => account["delegate"] || pk,
      "zkapp" => normalize_zkapp(account["zkapp"])
    }
end

sorted_result = result.sort.to_h
final_result = sorted_result.transform_values { |v| v.sort.to_h }

puts JSON.pretty_generate(final_result.sort.to_h)
