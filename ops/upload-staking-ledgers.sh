#!/bin/sh

# Uploads Mina staking ledger logs to the Granola bucket.
# $1 = source directory

set -x
set -eu

RESULT='Failure.'
exit_handler() {
	echo "${RESULT} Exiting." >&2
}
trap exit_handler EXIT

MY_DIR="$(CDPATH= cd "$(dirname "$0")" && pwd)"

# Verify that we can connect.
"$MY_DIR"/granola-rclone.rb lsd linode:

# Perform the upload.
"$MY_DIR"/granola-rclone.rb \
	sync \
	--metadata \
	"$1" \
	linode:granola-mina-staking-ledgers
RESULT='Staking ledgers uploaded.'
