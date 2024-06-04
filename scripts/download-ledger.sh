#!/usr/bin/env bash

destdir="$1"
epoch="$2"
ledger_hash="$3"

curl \
  https://storage.googleapis.com/mina-explorer-ledgers/"$ledger_hash".json \
  -o "$destdir"/mainnet-"$epoch"-"$ledger_hash".json
