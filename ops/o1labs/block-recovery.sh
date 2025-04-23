#! /bin/sh

# $1 = network
# $2 = block height
# $3 = destination

set -eu

MY_DIR="$(CDPATH='' cd "$(dirname "$0")" && pwd)"

exec "$MY_DIR"/download-mina-blocks.rb -d "$3" blocks "$2"
