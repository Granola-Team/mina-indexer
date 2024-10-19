#!/bin/sh

# Uploads Mina node logs of blocks (so-called "precomputed blocks") to the Granola store.
# $1 = order of magnitude of blocks desired (e.g. "4" means about 10,000 blocks)
# $2 = source directory

# For debugging, uncomment the following line:
# set -x

set -eu

RESULT='Failure.'
exit_handler() {
	echo "${RESULT} Exiting." >&2
}
trap exit_handler EXIT

MY_DIR="$(CDPATH= cd "$(dirname "$0")" && pwd)"

# Upload the Mina blocks logs to the Granola bucket. Note that "$1" is the
# order of magnitude of the block count. So, "3" gets blocks with height up to
# 99, and "5" gets blocks with height up to 9999. "$2" is the source directory.
#

# Verify that we can connect.
"$MY_DIR"/granola-rclone.rb lsd linode:

# Perform the download.
X="$(expr $1 + 1)"
"$MY_DIR"/granola-rclone.rb \
	sync \
	"$2" \
	linode:granola-mina-blocks \
	--exclude '*mainnet-{{\d{'"$X"',}}}-*' \
	--dump filters
RESULT='Blocks uploaded.'
