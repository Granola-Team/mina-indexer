#! /usr/bin/env -S ruby -w
# frozen_string_literal: true
# -*- mode: ruby -*-

require 'json'

filename = ARG[0]
file = File.read(filename)
data = JSON.parse(file)

accounts = data['ledger']['accounts']

result = {}

accounts.each do |account|
  public_key = account['pk']
  nonce = account['nonce'] || '0' # Default to 'N/A' if nonce is not present
  balance = account['balance']

  result[public_key] = { 
    'nonce' => nonce,
    'balance' => balance
  }
end

sorted_result = result.sort.to_h

puts JSON.pretty_generate(sorted_result)
