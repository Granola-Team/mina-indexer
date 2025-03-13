#!/bin/sh

# Upload files from the given directory to Granola's bucket for stripped
# precomputed blocks.

# For debugging, uncomment the following line:
# set -x

set -eu

RESULT='Failure.'
exit_handler() {
    echo "${RESULT} Exiting." >&2
}
trap exit_handler EXIT

MY_DIR="$(CDPATH= cd "$(dirname "$0")" && pwd)"

# Perform the download.
"$MY_DIR"/granola-rclone.rb sync "$1" linode-granola:granola-mina-stripped-blocks/mina-blocks
RESULT='Blocks uploaded.'
