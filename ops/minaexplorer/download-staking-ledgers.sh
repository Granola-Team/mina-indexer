#! /bin/sh

set -eu

DEST="$1"

MYDIR="$(CDPATH= cd "$(dirname "$0")" && pwd)"

cat "$MYDIR"/ledgers.json |
	jq -r '.[][]' |
	while read -r epoch; do
		read -r h
		curl https://storage.googleapis.com/mina-explorer-ledgers/"$h".json \
			-o "$DEST"/mainnet-"$epoch"-"$h".json
	done
