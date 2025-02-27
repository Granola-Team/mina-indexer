#!/bin/sh

# For debugging, uncomment the following line:
# set -x

set -eu

RESULT='Failure.'
exit_handler() {
	echo "${RESULT} Exiting." >&2
}
trap exit_handler EXIT

MY_DIR="$(CDPATH= cd "$(dirname "$0")" && pwd)"

"$MY_DIR"/granola-rclone.rb copyto "$1" linode-granola:granola-mina-indexer-snapshots/mina-indexer-snapshots/"$1"
RESULT='Success.'
