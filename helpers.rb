require 'fileutils'
require 'random/formatter'

# For debugging
def dbg s
  puts s
end

seed = Integer(ARGV[0])
pnrg = Random::new(seed)

SRC_TOP = `git rev-parse --show-toplevel`.strip
HOME_DIR = ENV["HOME"] + "/mina-indexer"
CURRENT = HOME_DIR + "/CURRENT"
VERSION = "stage-" + pnrg.hex(3)
BASE_DIR = HOME_DIR + "/" + VERSION
EXE = BASE_DIR + "/bin/mina-indexer"
SOCKET = BASE_DIR + "/mina-indexer.socket"
LOGS_DIR = BASE_DIR + "/logs"
LEDGERS_DIR = BASE_DIR + "/staking-ledgers"
BLOCKS_DIR = BASE_DIR + "/blocks"
DB_DIR = BASE_DIR + "/db"

def randomPort
  rand 10000..50000
end

def createBaseDir
  FileUtils.mkdir_p BASE_DIR
end

def destroyBaseDir
  FileUtils.rm_rf BASE_DIR
end

def configLogDir
  FileUtils.mkdir_p LOGS_DIR
end

def configExecDir
  FileUtils.mkdir_p BASE_DIR + "/bin"
  idxr = SRC_TOP + '/rust/target/release/mina-indexer'
  FileUtils.cp idxr, EXE, :verbose => true
end

def getLedgers
  success = system SRC_TOP + "/ops/download-staking-ledgers " + LEDGERS_DIR
  if ! success
    abort "Something went wrong with staking ledger downloads."
  end
end

def getBlocks magnitude
  success = system SRC_TOP +
    "/ops/download-mina-blocks " + magnitude.to_s + " " + BLOCKS_DIR
  if ! success
    abort "Downloading Mina blocks failed."
  end
end
