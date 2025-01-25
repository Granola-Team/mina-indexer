#!/bin/sh

# Uploads Mina node logs of blocks (so-called "precomputed blocks") to the Granola bucket.
# $1 = order of magnitude of blocks desired. Examples:
#   3 → blocks up to 999
#   4 → blocks up to 9999
#   5 → blocks up to 99999
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

# Verify that we can connect.
"$MY_DIR"/granola-rclone.rb lsd cloudflare:

# Perform the download.
X="$(expr $1 + 1)"
"$MY_DIR"/granola-rclone.rb \
	sync \
	"$2" \
	cloudflare:mina-blocks \
	--exclude '*mainnet-{{\d{'"$X"',}}}-*' \
RESULT='Blocks uploaded.'
