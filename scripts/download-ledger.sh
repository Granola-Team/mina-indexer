#!/usr/bin/env bash

ledger_hash=$1
epoch=$2

curl https://storage.googleapis.com/mina-explorer-ledgers/$ledger_hash.json -o ~/.mina-indexer/staking-ledgers/mainnet-$epoch-$ledger_hash.json