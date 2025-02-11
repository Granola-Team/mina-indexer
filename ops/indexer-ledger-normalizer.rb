#!/usr/bin/env -S ruby -w

# -*- mode: ruby -*-

require "json"

# All token ledgers
data = JSON.parse(File.read(ARGV[0]))

# Get the MINA token ledger
mina_ledger = data["wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf"]

result = {}

mina_ledger.each_value do |value|
  pk = value["public_key"]
  nonce = (value["nonce"] || 0).to_s
  balance = value["balance"].to_s
  delegate = value["delegate"] || pk
  result[pk] = {
    "nonce" => nonce,
    "balance" => balance,
    "delegate" => delegate
  }
end

puts JSON.pretty_generate(result.sort.to_h)
