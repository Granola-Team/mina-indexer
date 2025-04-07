#!/usr/bin/env -S ruby -w

require "json"
require "#{__dir__}/recursive-sort-hash.rb"

MINA_TOKEN = "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf"

filename = ARGV[0]
file = File.read(filename)
accounts = JSON.parse(file)

# Handle both ledger styles
accounts = accounts["ledger"]["accounts"] if !accounts.instance_of?(Array)

# TODO: employ the below to check against zkApp ledger data.
#
# def normalize_zkapp(data)
#   return data if data.nil?
#
#   zkapp = data
#   zkapp["app_state"] = data["app_state"].map { |app| normalize_hex(app) }
#   zkapp["action_state"] = data["action_state"].map { |action| normalize_hex(action) }
#   zkapp["last_action_slot"] = data["last_action_slot"].to_s
#   zkapp
# end
#
# def normalize_hex(data)
#   hex_str = data.to_i.to_s(16).upcase
#   "0x" + hex_str.ljust(64, "0")
# end

result = {}

# Normalize all token accounts
accounts.each do |account|
  pk = account["pk"]
  raise "Missing public key: #{JSON.pretty_generate(account)}" if pk.nil?

  token = account["token"]
  if token.nil? || token == "1"
    # The MINA token was, pre-hardfork, token 1.
    token = MINA_TOKEN
  end

  result[token] ||= {}
  result[token][pk] =
    {
      "nonce" => account["nonce"] || "0",
      "balance" => account["balance"],
      "delegate" => account["delegate"] || pk

      # TODO: check against zkApp ledger data.
      # "zkapp" => normalize_zkapp(account["zkapp"])
    }
end

puts JSON.pretty_generate(sort_recursively(result))
