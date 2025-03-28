#!/usr/bin/env -S ruby -w

require "json"

MINA_TOKEN = "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf"

# All token ledgers
data = JSON.parse(File.read(ARGV[0]))

result = {}

data.keys.each do |token|
  data[token].each_value do |value|
    pk = value["public_key"]
    result[token] ||= {}
    result[token][pk] =
      {
        "nonce" => (value["nonce"] || 0).to_s,
        "balance" => value["balance"].to_s,
        "delegate" => value["delegate"] || pk
      }
  end
end

sorted_result = result.sort.to_h
final_result = sorted_result.transform_values { |v| v.sort.to_h }
puts JSON.pretty_generate(final_result.sort.to_h)
