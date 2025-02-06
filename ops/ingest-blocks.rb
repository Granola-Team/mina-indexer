#!/usr/bin/env -S ruby -w

# -*- mode: ruby -*-

DEPLOY_TYPE = ARGV[0]       # 'test' or 'prod'
BUILD_TYPE = ARGV[1]        # 'nix' or 'debug'
BLOCKS_COUNT = ARGV[2]      # number of blocks to deploy

require "#{__dir__}/ops-common"

puts "Ingesting blocks (mode: #{DEPLOY_TYPE}) with #{BLOCKS_COUNT} blocks."

# Configure the directories as needed.
config_exe_dir
config_log_dir
get_blocks BLOCKS_COUNT
fetch_ledgers

# Create the database, if needed.
if File.exist?(db_dir(BLOCKS_COUNT))
  puts "Blocks already ingested for #{BLOCKS_COUNT} blocks."
  exit 0
else
  puts "Ingesting Blocks..."
  command = [EXE,
    "database", "ingest",
    "--database-dir", db_dir(BLOCKS_COUNT),
    "--blocks-dir", blocks_dir(BLOCKS_COUNT)]
  puts command.join(" ")
  success = system(*command)
  puts success ? "Block ingestion complete." : "Block ingestion failed."

  exit success ? 0 : 1
end
