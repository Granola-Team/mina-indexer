#!/bin/sh

# Uploads Mina staking ledger logs to the Granola bucket. Requires appropriate
# credentials.
#
# $1 = source directory

set -eu

RESULT='Failure.'
exit_handler() {
	echo "${RESULT} Exiting." >&2
}
trap exit_handler EXIT

MY_DIR="$(CDPATH='' cd "$(dirname "$0")" && pwd)"

# Perform the upload.
"$MY_DIR"/granola-rclone.rb \
	sync \
	--metadata \
	"$1" \
	linode-granola:staking-ledgers.minasearch.com
RESULT='Staking ledgers uploaded.'
