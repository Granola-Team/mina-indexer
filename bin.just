# Variables

SRC := `git rev-parse --show-toplevel`
REV := `git rev-parse --short=8 HEAD`
VOLUMES_DIR := env_var_or_default('VOLUMES_DIR', '/mnt')
DEV_DIR := VOLUMES_DIR + "/mina-indexer-dev"
BASE_DIR := DEV_DIR + "/" + REV
PID_FILE := BASE_DIR + "/idxr_pid"
SOCKET_FILE := BASE_DIR + "/mina-indexer.sock"
BLOCKS_DIR := BASE_DIR + "/blocks"
DATABASE_DIR := BASE_DIR + "/database"
STAKING_LEDGERS_DIR := BASE_DIR + "/staking-ledgers"
V2_GENESIS_STATE_HASH := "3NK4BpDSekaqsG6tx8Nse2zJchRft2JpnbvMiog55WCr5xJZaKeP"

# Environment variables

export RUST_BACKTRACE := "full"

# List available recipes
list:
    @echo "Mina Indexer bin - available recipes:"
    @just -f {{ source_file() }} --list

# Run indexer with specified arguments
idxr-bin IDXR_BIN *args:
    #!/usr/bin/env bash
    set -euo pipefail
    cd {{ BASE_DIR }}
    {{ IDXR_BIN }} --socket {{ SOCKET_FILE }} {{ args }}

# Helper functions for shutdown check

_socket_exists := "test -S " + SOCKET_FILE
_pid_file_exists := "test -e " + PID_FILE
_process_running := "ps -p $(cat " + PID_FILE + ") > /dev/null 2>&1"

# Shutdown the indexer gracefully
shutdown IDXR_BIN:
    echo "Shutting down Mina Indexer."

    if ! {{ _socket_exists }}; then \
        echo "  Missing socket. Shutdown failed."; \
        exit 1; \
    fi

    if ! {{ _pid_file_exists }}; then \
        echo "  Missing PID file. Shutdown failed."; \
        exit 1; \
    fi

    if ! {{ IDXR_BIN }} --socket {{ SOCKET_FILE }} server shutdown; then \
        echo "  Server shutdown command failed."; \
        exit 1; \
    fi

    sleep 3

    if {{ _process_running }}; then \
        echo "  The shutdown command did not kill the process. Failure."; \
        exit 1; \
    else \
        if {{ _socket_exists }}; then \
            echo "  The shutdown command did not delete the socket. Failure."; \
            exit 1; \
        fi \
    fi

    echo "Deleting PID file."
    rm -f {{ PID_FILE }}
    sleep 2

# Find an ephemeral port
ephemeral-port:
    @ruby -e 'require "socket"; low_bound=49152; range=16384; loop do; candidate=low_bound+rand(range); begin; TCPSocket.new("127.0.0.1", candidate).close; rescue Errno::ECONNREFUSED; puts candidate; break; end; end'

# Wait for socket with timeout
wait-for-socket max_retries="250":
    @ruby -e 'num_retries=0; max={{ max_retries }}; while num_retries < max.to_i; if File.socket?("{{ SOCKET_FILE }}"); exit(0); end; num_retries += 1; puts "Sleeping (#{num_retries})..."; sleep(1); end; exit(1)'

# Wait indefinitely for socket
wait-forever-for-socket:
    @ruby -e 'loop do; if File.socket?("{{ SOCKET_FILE }}"); exit(0); end; puts "Sleeping 10s..."; sleep(10); end'

# Create an indexer database
[private]
database-create IDXR_BIN *args:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p {{ BASE_DIR }}
    cd {{ BASE_DIR }}
    {{ IDXR_BIN }} --socket {{ SOCKET_FILE }} database create \
        --blocks-dir {{ BLOCKS_DIR }} \
        --staking-ledgers-dir {{ STAKING_LEDGERS_DIR }} \
        --database-dir {{ DATABASE_DIR }}

# Start the indexer server and wait for socket (with timeout)
start IDXR_BIN *args:
    #!/usr/bin/env bash
    set -euo pipefail
    mkdir -p {{ BASE_DIR }}
    cd {{ BASE_DIR }}
    {{ IDXR_BIN }} --socket {{ SOCKET_FILE }} server start {{ args }} & echo $! > {{ PID_FILE }}
    just -f {{ source_file() }} wait-for-socket

# Start the indexer server with an ephemeral port
_start IDXR_BIN *args:
    #!/usr/bin/env bash
    set -euo pipefail
    port=`just -f {{ source_file() }} ephemeral-port`
    just -f {{ source_file() }} start {{ IDXR_BIN }} \
        --web-port $port \
        --blocks-dir {{ BLOCKS_DIR }} \
        --staking-ledgers-dir {{ STAKING_LEDGERS_DIR }} \
        --database-dir {{ DATABASE_DIR }} \
        {{ args }}

# Create and start a v1 database/server with `blocks-dir`, `staking-ledgers-dir`, and `database-dir`
start-v1 IDXR_BIN *args:
    @echo "Creating v1 mina indexer database"
    @just -f {{ source_file() }} database-create {{ IDXR_BIN }}

    @echo "Starting mina indexer server from v1 database"
    @just -f {{ source_file() }} _start {{ IDXR_BIN }} \
        {{ args }}

# Create and start a v2 database/server with `blocks-dir`, `staking-ledgers-dir`, and `database-dir`
start-v2 IDXR_BIN *args:
    @echo "Creating v2 mina indexer database"
    @just -f {{ source_file() }} database-create {{ IDXR_BIN }} \
        --genesis-hash {{ V2_GENESIS_STATE_HASH }}

    @echo "Starting mina indexer server from v2 database"
    @just -f {{ source_file() }} _start {{ IDXR_BIN }} \
        --genesis-hash {{ V2_GENESIS_STATE_HASH }} \
        {{ args }}

# Create and start a v2 database/server
stage-and-start-v2 IDXR_BIN block_height *args:
    #!/usr/bin/env bash
    set -euo pipefail
    just -f {{ source_directory() + "/stage-blocks.just" }} v2 {{ block_height }} {{ BLOCKS_DIR }}

    just -f {{ source_file() }} database-create {{ IDXR_BIN }} \
        --genesis-hash {{ V2_GENESIS_STATE_HASH }}

    port=`just -f {{ source_file() }} ephemeral-port`

    just -f {{ source_file() }} start {{ IDXR_BIN }} \
        --web-port "$port" \
        --web-hostname "0.0.0.0" \
        --blocks-dir {{ BLOCKS_DIR }} \
        --staking-ledgers-dir {{ STAKING_LEDGERS_DIR }} \
        --database-dir {{ DATABASE_DIR }} \
        --genesis-hash {{ V2_GENESIS_STATE_HASH }} \
        {{ args }}
