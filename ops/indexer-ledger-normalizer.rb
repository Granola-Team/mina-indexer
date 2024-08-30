#! /usr/bin/env -S ruby -w
# frozen_string_literal: true

# -*- mode: ruby -*-

require 'json'

filename = ARGV[0]
file_content = File.read(filename)
data = JSON.parse(file_content)

result = {}

data.each_value do |value|
  balance_nanomina = value['balance'].to_s.rjust(10, '0')
  balance_mina = "#{balance_nanomina[0..-10]}.#{balance_nanomina[-9..]}"
  normalized_balance = balance_mina.sub(/\.?0+$/, '')
  nonce = (value['nonce'] || 0).to_s

  result[value['public_key']] = {
    'nonce' => nonce,
    'balance' => normalized_balance
  }
end

sorted_result = result.sort.to_h

puts JSON.pretty_generate(sorted_result)
