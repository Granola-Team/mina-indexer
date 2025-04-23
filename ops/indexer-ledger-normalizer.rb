#!/usr/bin/env -S ruby -w

require "json"
require "#{__dir__}/recursive-sort-hash.rb"

data = JSON.parse(File.read(ARGV[0]))

result = {}
data.keys.each do |token|
  data[token].each_value do |account|
    pk = account["public_key"]
    result[token] ||= {}
    result[token][pk] = {
      "nonce" => (account["nonce"] || 0).to_s,
      "balance" => account["balance"].to_s,
      "delegate" => account["delegate"] || pk,
      "zkapp" => account["zkapp"]
    }
  end
end

def remove_vk_actions(obj)
  case obj
  when Hash
    obj.delete("action_state")
    obj.delete("proved_state")
    obj.delete("verification_key")
    obj.each_value { |v| remove_vk_actions(v) }
  when Array
    obj.each { |item| remove_vk_actions(item) }
  end
  obj
end

puts JSON.pretty_generate(sort_recursively(remove_vk_actions(result)))
