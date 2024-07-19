#! /usr/bin/env -S ruby -w
# frozen_string_literal: true
# -*- mode: ruby -*-

require 'json'

filename = ARG[0]
file_content = File.read(filename)
data = JSON.parse(file_content)

result = {}

data.each do |key, value|
  balance_nanomina = value['balance'].to_s.rjust(10, '0')
  balance_mina = balance_nanomina[0..-10] + '.' + balance_nanomina[-9..-1]
  normalized_balance = balance_mina.sub(/\.?0+$/, '')

  result[value['public_key']] = { 
    'nonce' => value['nonce'].to_s, 
    'balance' => normalized_balance
  }
end

sorted_result = result.sort.to_h

puts JSON.pretty_generate(sorted_result)
