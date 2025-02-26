#!/bin/sh

# For debugging, uncomment the following line:
set -x

set -eu

RESULT='Failure.'
exit_handler() {
	echo "${RESULT} Exiting." >&2
}
trap exit_handler EXIT

MY_DIR="$(CDPATH= cd "$(dirname "$0")" && pwd)"

# Verify that we can connect.
"$MY_DIR"/granola-rclone.rb lsd linode-granola-mina-indexer-snapshots:

"$MY_DIR"/granola-rclone.rb copyto "$1" linode-granola-mina-indexer-snapshots:mina-indexer-snapshots/"$1"
RESULT='Success.'
