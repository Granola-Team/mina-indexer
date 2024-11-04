#!/usr/bin/env -S ruby -w

# -*- mode: ruby -*-

DEPLOY_TYPE = ARGV[0]       # 'test' or 'prod'
BUILD_TYPE = ARGV[1]        # 'nix' or 'debug'
BLOCKS_COUNT = ARGV[2]      # number of blocks to deploy

VOLUMES_DIR = ENV["VOLUMES_DIR"] || "/mnt"
BASE_DIR = "#{VOLUMES_DIR}/mina-indexer-#{DEPLOY_TYPE}"

require "fileutils"
require "#{__dir__}/helpers" # Expects BASE_DIR to be defined

abort "Error: #{BASE_DIR} must exist to perform the deployment." unless File.exist?(BASE_DIR)

puts "Deploying (#{DEPLOY_TYPE}) with #{BLOCKS_COUNT} blocks."

# Configure the directories as needed.
config_base_dir
config_exe_dir
config_log_dir
get_blocks BLOCKS_COUNT
fetch_ledgers

# Create the database, if needed.
if File.exist?(db_dir(BLOCKS_COUNT))
  puts "Database already exists for #{BLOCKS_COUNT} blocks."
  exit 0
else
  puts "Creating database..."

  if BUILD_TYPE == "debug"
    puts "Ingest staking ledgers? (y/n)"
    ingest_staking_ledgers = $stdin.gets[0].downcase
    until ["n", "y"].include? ingest_staking_ledgers
      warn("Invalid response")
      puts "Ingest staking ledgers? (y/n)"
      ingest_staking_ledgers = $stdin.gets[0].downcase
    end

    puts "Ingest orphan blocks? (y/n)"
    ingest_orphan_blocks = $stdin.gets[0].downcase
    until ["n", "y"].include? ingest_orphan_blocks
      warn("Invalid response")
      puts "Ingest orphan blocks? (y/n)"
      ingest_orphan_blocks = $stdin.gets[0].downcase
    end

    ingest_staking_ledgers = ingest_staking_ledgers == "y"
    ingest_orphan_blocks = ingest_orphan_blocks == "y"

    success = if !ingest_staking_ledgers && !ingest_orphan_blocks
      system(
        EXE,
        "database", "create",
        "--log-level", "DEBUG",
        "--ledger-cadence", "5000",
        "--database-dir", db_dir(BLOCKS_COUNT),
        "--blocks-dir", blocks_dir(BLOCKS_COUNT),
        "--do-not-ingest-orphan-blocks"
      )
    elsif !ingest_staking_ledgers && ingest_orphan_blocks
      system(
        EXE,
        "database", "create",
        "--log-level", "DEBUG",
        "--ledger-cadence", "5000",
        "--database-dir", db_dir(BLOCKS_COUNT),
        "--blocks-dir", blocks_dir(BLOCKS_COUNT)
      )
    elsif ingest_staking_ledgers && !ingest_orphan_blocks
      system(
        EXE,
        "database", "create",
        "--log-level", "DEBUG",
        "--ledger-cadence", "5000",
        "--database-dir", db_dir(BLOCKS_COUNT),
        "--blocks-dir", blocks_dir(BLOCKS_COUNT),
        "--staking-ledgers-dir", LEDGERS_DIR,
        "--do-not-ingest-orphan-blocks"
      )
    else
      system(
        EXE,
        "database", "create",
        "--log-level", "DEBUG",
        "--ledger-cadence", "5000",
        "--database-dir", db_dir(BLOCKS_COUNT),
        "--blocks-dir", blocks_dir(BLOCKS_COUNT),
        "--staking-ledgers-dir", LEDGERS_DIR
      )
    end
  else
    success = system(
      EXE,
      "database", "create",
      "--log-level", "DEBUG",
      "--ledger-cadence", "5000",
      "--database-dir", db_dir(BLOCKS_COUNT),
      "--blocks-dir", blocks_dir(BLOCKS_COUNT),
      "--staking-ledgers-dir", LEDGERS_DIR
    )
  end
  puts success ? "Database creation succeeded." : "Database creation failed."

  exit success ? 0 : 1
end
