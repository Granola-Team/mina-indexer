#!/usr/bin/env -S ruby -w

require "json"

# All token ledgers
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
        "delegate" => account["delegate"] || pk,
        "zkapp" => account["zkapp"]
      }
  end
end

sorted_result = result.sort.to_h
final_result = sorted_result.transform_values { |v| v.sort.to_h }
puts JSON.pretty_generate(final_result.sort.to_h)
