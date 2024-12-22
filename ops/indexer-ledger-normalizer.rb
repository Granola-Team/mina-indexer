#!/usr/bin/env -S ruby -w

# -*- mode: ruby -*-

require "json"

filename = ARGV[0]
file_content = File.read(filename)

# All token ledgers
data = JSON.parse(file_content)

# Get the MINA token ledger
mina_ledger = data["wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf"]

result = {}

mina_ledger.each_value do |value|
  balance_nanomina = value["balance"].to_s.rjust(10, "0")
  balance_mina = "#{balance_nanomina[0..-10]}.#{balance_nanomina[-9..]}"
  normalized_balance = balance_mina.sub(/\.?0+$/, "")
  nonce = (value["nonce"] || 0).to_s
  delegate = value["delegate"] || value["public_key"]
  result[value["public_key"]] = {
    "nonce" => nonce,
    "balance" => normalized_balance,
    "delegate" => delegate
  }
end

sorted_result = result.sort.to_h

puts JSON.pretty_generate(sorted_result)
