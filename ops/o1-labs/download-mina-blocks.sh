#!/bin/sh

set -ex

MIN_HEIGHT="$1"
MAX_HEIGHT="$2"
BLOCKS_DIR="$3"
URLS_LIST="$BLOCKS_DIR"/block_urls
mkdir -p "$BLOCKS_DIR"

for BLOCK_HEIGHT in $(seq "$MIN_HEIGHT" "$MAX_HEIGHT"); do
	echo "gs://mina_network_block_data/mainnet-${BLOCK_HEIGHT}-*.json" \
		>>"$URLS_LIST"
done

exit_handler() {
	rm -f "$URLS_LIST"
}
trap exit_handler EXIT

gsutil -m cp -n -I "$BLOCKS_DIR" <"$URLS_LIST"
