#!/usr/bin/env -S ruby -w

START_BLOCK = ARGV[0].to_i
END_BLOCK = ARGV[1].to_i
NETWORK = ARGV[2]
DEST = ARGV[3]

BUILD_TYPE = "dev"
DEPLOY_TYPE = "dev"
REV = "dummy-revision"
SRC_TOP = "/nowhere"
require "#{__dir__}/ops-common"

stage_blocks(END_BLOCK, START_BLOCK, NETWORK, DEST)
