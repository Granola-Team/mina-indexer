require 'fileutils'

# Constants

SRC_TOP = `git rev-parse --show-toplevel`.strip
REV = `git rev-parse --short=8 HEAD`.strip
CURRENT = BASE_DIR + '/CURRENT'

# Port

def randomPort
  rand 10000..50000
end

# Base directory

def createBaseDir
  FileUtils.mkdir_p BASE_DIR
end

def destroyBaseDir
  FileUtils.rm_rf BASE_DIR
end

# Logs

LOGS_DIR = BASE_DIR + '/logs/' + REV

def configLogDir
  FileUtils.mkdir_p LOGS_DIR
end

# Snapshots

SNAPSHOT_DIR = BASE_DIR + '/snapshots/' + REV

def configSnapshotDir
  # FileUtils.mkdir_p SNAPSHOT_DIR
  # ^ Do not do this because it will cause mina-indexer to fail to create the
  #   snapshot.
end

# Executable

EXE_DIR = BASE_DIR + '/bin'
EXE = EXE_DIR + '/mina-indexer-' + REV

def configExecDir
  FileUtils.mkdir_p EXE_DIR
  idxr = SRC_TOP + '/result/bin/mina-indexer'
  FileUtils.cp idxr, EXE

  # Add the write bit in case we need to overwrite, later.
  File.chmod(0700, EXE)
end

# Socket

SOCKET = BASE_DIR + '/mina-indexer-' + REV + '.socket'

def waitForSocket(waitInterval)
  waitSeconds = 0
  until File.exist?(SOCKET) do
    puts "Waited #{waitSeconds} s for #{SOCKET}..."
    sleep waitInterval
    waitSeconds += waitInterval
  end
end

# Ledgers 

LEDGERS_DIR = BASE_DIR + "/staking-ledgers"

def getLedgers
  system(SRC_TOP + '/ops/download-staking-ledgers', LEDGERS_DIR) ||
    abort('Something went wrong with staking ledger downloads.')
end

# Blocks

BLOCKS_DIR = BASE_DIR + "/blocks"

def getBlocks(block_count)
  system(
    SRC_TOP + '/ops/download-mina-blocks.rb',
    '1',               # start block
    block_count.to_s,  # end block
    BLOCKS_DIR
  ) || abort('Downloading Mina blocks failed.')
end

# Database directory

DB_VERSION = '0.5.0'
DB_DIR = BASE_DIR + '/db/' + DB_VERSION
