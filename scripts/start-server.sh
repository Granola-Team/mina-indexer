#!/usr/bin/env bash

#BLOCKS_DIR=~/blocks/15000
BLOCKS_DIR=./blocks
STAKING_LEDGERS_DIR=~/.mina-indexer/staking-ledgers
GENESIS_LEDGERS_DIR=~/.mina-indexer/genesis-ledgers
DATABASE_DIR=~/.mina-indexer/database

IDXR="$(pwd)"/rust/target/release/mina-indexer
DOMAIN_SOCKET_PATH="$(pwd)"/rust/mina-indexer.sock

mina-indexer() {
    "$IDXR" "$@"
}

mina-indexer server start \
        --blocks-dir "$BLOCKS_DIR" \
        --staking-ledgers-dir "$STAKING_LEDGERS_DIR" \
        --database-dir "$DATABASE_DIR" \
        --log-level debug 2>&1