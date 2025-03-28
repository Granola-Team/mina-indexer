#!/usr/bin/env -S ruby -w

require "json"

MINA_TOKEN = "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf"

filename = ARGV[0]
file = File.read(filename)
accounts = JSON.parse(file)

# for genesis ledgers
# data = JSON.parse(file)
# accounts = data["ledger"]["accounts"]

result = {}

# Normalize all token accounts
accounts.each do |account|
  pk = account["pk"]
  raise "Missing public key: #{JSON.pretty_generate(account)}" if pk.nil?

  token = account["token"] || MINA_TOKEN
  result[token] ||= {}
  result[token][pk] =
    {
      "nonce" => account["nonce"] || "0",
      "balance" => account["balance"],
      "delegate" => account["delegate"] || pk
    }
end

sorted_result = result.sort.to_h
final_result = sorted_result.transform_values { |v| v.sort.to_h }

puts JSON.pretty_generate(final_result.sort.to_h)
