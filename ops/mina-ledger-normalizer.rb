#!/usr/bin/env -S ruby -w

# -*- mode: ruby -*-

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
  balance = account["balance"]
  nonce = account["nonce"] || "0"
  delegate = account["delegate"] || pk
  token = account["token"] || MINA_TOKEN

  result[pk] = if token == MINA_TOKEN
    # don't add MINA token
    {
      "nonce" => nonce,
      "balance" => balance,
      "delegate" => delegate
    }
  else
    # add non-MINA token
    {
      "nonce" => nonce,
      "balance" => balance,
      "delegate" => delegate,
      "token" => token
    }
  end
end

sorted_result = result.sort.to_h

puts JSON.pretty_generate(sorted_result)
