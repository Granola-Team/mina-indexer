#!/usr/bin/env -S ruby -w

# -*- mode: ruby -*-

START_BLOCK = ARGV[0].to_i
END_BLOCK = ARGV[1].to_i
NETWORK = ARGV[2]
DEST = ARGV[3]

BUILD_TYPE = "dev"
DEPLOY_TYPE = "dev"
REV = "dummy-revision"
SRC_TOP = "/nowhere"
require "#{__dir__}/ops-common"

args = [
  "#{__dir__}/download-mina-blocks.rb",
  START_BLOCK.to_s,
  END_BLOCK.to_s,
  MASTER_BLOCKS_DIR
]
warn "stage-blocks.rb issuing: #{args}"
system(*args) || abort("Failure of download-mina-blocks.rb")

# Copy (or hard link) the correct block files into the staging directory.
#
FileUtils.mkdir_p(DEST)
print "Staging blocks into #{DEST}... "
(START_BLOCK..END_BLOCK).each do |block_height|
  Dir["#{MASTER_BLOCKS_DIR}/#{NETWORK}-#{block_height}-*.json"].each do |src|
    print "." # To show progress
    target = "#{DEST}/#{File.basename(src)}"
    unless File.exist?(target)
      # Use hard links to avoid uselessly overfilling the storage.
      File.link(src, target)
    end
  end
end
puts
