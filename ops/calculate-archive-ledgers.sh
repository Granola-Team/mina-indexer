#!/bin/sh

# Use this script to generate ledgers from the AN. This is useful for
# us to compare ledgers with the indexer.
#
# Usage: set a state_hash for which you want to generate a ledger for
# and the connection_string to talk to an archive node

mina_replayer=$(which mina-replayer)
state_hash=
connection_string=

curl -s "https://raw.githubusercontent.com/MinaProtocol/mina/compatible/genesis_ledgers/mainnet.json" | jq '{target_epoch_ledgers_state_hash:"'$state_hash'",genesis_ledger:{add_genesis_winner: true, accounts: .ledger.accounts}}' >replayer_input.json

$mina_replayer --checkpoint-interval 1 --archive-uri $connection_string --input-file replayer_input.json --output-file /dev/null --continue-on-error >replayer.log
