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

LOGS_DIR = BASE_DIR + '/logs' + REV

def configLogDir
  FileUtils.mkdir_p LOGS_DIR
end

# Executable

EXE_DIR = BASE_DIR + '/bin'
EXE = EXE_DIR + '/' + REV

def configExecDir
  FileUtils.mkdir_p EXE_DIR
  idxr = SRC_TOP + '/result/bin/mina-indexer'
  FileUtils.cp idxr, EXE
end

# Socket

SOCKET = BASE_DIR + '/mina-indexer-' + REV + '.socket'

# Ledgers 

LEDGERS_DIR = BASE_DIR + "/staking-ledgers"

def getLedgers
  unless system(SRC_TOP + '/ops/download-staking-ledgers', LEDGERS_DIR) do
    abort "Something went wrong with staking ledger downloads."
  end
end

# Blocks

BLOCKS_DIR = BASE_DIR + "/blocks"

def getBlocks magnitude
  unless system(
    SRC_TOP + '/ops/download-mina-blocks',
    magnitude.to_s,
    BLOCKS_DIR
  ) do
    abort "Downloading Mina blocks failed."
  end
end

# Database directory

DB_VERSION = '0.1.1'
DB_DIR = BASE_DIR + '/db' + DB_VERSION
