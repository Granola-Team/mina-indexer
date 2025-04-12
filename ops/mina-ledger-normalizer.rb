#!/usr/bin/env -S ruby -w

require "json"
require "#{__dir__}/recursive-sort-hash.rb"

MINA_TOKEN = "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf"

filename = ARGV[0]
file = File.read(filename)
accounts = JSON.parse(file)

# Handle both ledger styles
accounts = accounts["ledger"]["accounts"] if !accounts.instance_of?(Array)

def normalize_hex(data)
  hex_str = data.to_i.to_s(16).upcase
  "0x" + hex_str.rjust(64, "0")
end

def normalize_zkapp(zkapp)
  unless zkapp.nil?
    zkapp["app_state"] = zkapp["app_state"].map { |app| normalize_hex(app) }
    zkapp["last_action_slot"] = zkapp["last_action_slot"].to_s
  end
  zkapp
end

result = {}
accounts.each do |account|
  pk = account["pk"]
  raise "Missing public key: #{JSON.pretty_generate(account)}" unless pk

  token = account["token"]
  # The MINA token was, pre-hardfork, token 1.
  token = MINA_TOKEN if token.nil? || token == "1"

  result[token] ||= {}
  result[token][pk] = {
    "nonce" => account["nonce"] || "0",
    "balance" => account["balance"],
    "delegate" => account["delegate"] || pk,
    "zkapp" => normalize_zkapp(account["zkapp"])
  }
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
