#! /bin/sh

# $1 = network
# $2 = block height
# $3 = destination

set -eu

MY_DIR="$(CDPATH='' cd "$(dirname "$0")" && pwd)"
"$MY_DIR"/../ops/stage-blocks.rb "$2" "$2" "$1" "$3"
