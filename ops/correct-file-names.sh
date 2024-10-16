#!/bin/sh

set -eux

DIR="${1:-./mina_network_block_data}"
CDPATH= cd "$DIR"

get_height() {
	HASH="$1"
	FILE=mainnet-"$HASH".json
	jq -r .protocol_state.body.consensus_state.blockchain_length <"./${FILE}"
}

get_corrected_file_name() {
	HASH="$1"
	find . -name "mainnet-*-${HASH}*"
}

# Check all hashes for existence of the correct file.
#
while read HASH; do
	HEIGHT="$(get_height "$HASH")"
	FILE="$(get_corrected_file_name "$HASH")"
	DESIRED="./mainnet-${HEIGHT}-${HASH}.json"
	NAKED_FN="./mainnet-${HASH}.json"
	if [ -n "$FILE" ]; then
		# A corrected file name exists. Verify that it uses the correct height.
		if [ "$FILE" = "$DESIRED" ]; then
			# It uses the correct height. Ensure contents match.
			if ! diff -pwu "$NAKED_FN" "$DESIRED" 2>&1 >/dev/null; then
				echo "Contents did not match. Record diff."
				jq <"$NAKED_FN" >1
				jq <"$DESIRED" >2
				diff -pwu 1 2 >"${HASH}.diff" || true
				rm 1 2
			fi
		else
			# It does not use the correct height. Stop.
			echo "$FILE does not match ${DESIRED}"
			exit 1
		fi
	else
		# No corrected file name exists. Create it.
		cp mainnet-"${HASH}".json "$DESIRED"
	fi
done <../hashes.list

# get_height "$(head -1 hashes.list)"
