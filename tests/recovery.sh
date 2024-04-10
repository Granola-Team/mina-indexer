#! /bin/sh

set -eu

MY_DIR="$(CDPATH='' cd "$(dirname "$0")" && pwd)"
cp "$MY_DIR"/data/initial-blocks/"$1"-*-"$3".json "$4"
