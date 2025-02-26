#!/usr/bin/env -S ruby -w

# -*- mode: ruby -*-

require "json"

MINA_TOKEN = "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf"

# All token ledgers
data = JSON.parse(File.read(ARGV[0]))

result = {}

# Get all token accounts
data.keys.each do |token|
  data[token].each_value do |value|
    pk = value["public_key"]
    balance = value["balance"].to_s
    nonce = (value["nonce"] || 0).to_s
    delegate = value["delegate"] || pk

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
end

puts JSON.pretty_generate(result.sort.to_h)
