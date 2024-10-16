#!/usr/bin/env -S ruby -w

# -*- mode: ruby -*-

require "json"

filename = ARGV[0]
file = File.read(filename)
data = JSON.parse(file)

accounts = data["ledger"]["accounts"]

result = {}

accounts.each do |account|
  public_key = account["pk"]
  nonce = account["nonce"] || "0"
  balance = account["balance"]
  delegate = account["delegate"] || public_key
  result[public_key] = {
    "nonce" => nonce,
    "balance" => balance,
    "delegate" => delegate
  }
end

sorted_result = result.sort.to_h

puts JSON.pretty_generate(sorted_result)
