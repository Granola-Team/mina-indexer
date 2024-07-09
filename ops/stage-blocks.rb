#! /usr/bin/env -S ruby -w
# frozen_string_literal: true

# -*- mode: ruby -*-

START_BLOCK = ARGV[0].to_i
END_BLOCK = ARGV[1].to_i
NETWORK = ARGV[2]
DEST = ARGV[3]

require 'fileutils'

# Fetch the blocks into the blocks directory if they're not already there.
#
VOLUMES_DIR = ENV['VOLUMES_DIR'] || '/mnt'
DEV_DIR = "#{VOLUMES_DIR}/mina-indexer-dev"
FileUtils.mkdir_p(DEV_DIR)
block_count = [9999, END_BLOCK].max
BLOCKS_DIR = "#{DEV_DIR}/blocks-#{block_count}"
args = [
  "#{__dir__}/download-mina-blocks.rb",
  '1',
  block_count.to_s,
  BLOCKS_DIR
]
warn "stage-blocks.rb issuing: #{args}"
system(*args) || abort('Failure of download-mina-blocks')

# Copy (or hard link) the correct block files into the staging directory.
#
FileUtils.mkdir_p(DEST)
print "Staging blocks into #{DEST}... "
(START_BLOCK..END_BLOCK).each do |block_height|
  Dir["#{BLOCKS_DIR}/#{NETWORK}-#{block_height}-*.json"].each do |src|
    print '.' # To show progress
    target = "#{DEST}/#{File.basename(src)}"
    unless File.exist?(target)
      # Use hard links to avoid uselessly overfilling the storage.
      File.link(src, target)
    end
  end
end
puts
