#!/usr/bin/env -S ruby -w

require "json"
require "#{__dir__}/recursive-sort-hash.rb"

data = JSON.parse(File.read(ARGV[0]))

result = {}

data.keys.each do |token|
  data[token].each_value do |account|
    pk = account["public_key"]
    result[token] ||= {}
    result[token][pk] =
      {
        "nonce" => (account["nonce"] || 0).to_s,
        "balance" => account["balance"].to_s,
        "delegate" => account["delegate"] || pk

        # TODO: compare zkApp data with Mina node.
        # "zkapp" => account["zkapp"]
      }
  end
end

puts JSON.pretty_generate(sort_recursively(result))
