#!/bin/sh
set -ex

BLOCKS_DIR="./blocks"
touch "$BLOCKS_DIR"/block_urls

MIN_LENGTH=2
MAX_LENGTH=200

OUT_DIR=.

for LENGTH in `seq $MIN_LENGTH $MAX_LENGTH`
do
    echo "gs://mina_network_block_data/mainnet-${LENGTH}-*.json" >> "$BLOCKS_DIR"/block_urls
done 2>/dev/null

gsutil -m cp -n -I "$BLOCKS_DIR" < "$BLOCKS_DIR"/block_urls