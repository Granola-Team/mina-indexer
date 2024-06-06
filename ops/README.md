# ops

## Operations Utilities

Files in this directory:

```
./buildkite/ci-pipeline.yaml
./buildkite/tier2-pipeline.yaml
./calculate_archive_ledgers.sh
./correct_file_names
./download-mina-blocks
./download-staking-ledgers
./granola-rclone
./hashes.list
./ingest-all
./maybe-dups
./minaexplorer/download-staking-ledgers
./minaexplorer/ledgers.json
./o1-labs/download-mina-blocks
./productionize
./rclone.conf
./upload-mina-blocks
./upload-staking-ledgers
```

./buildkite
  Directory for settings for Buildkite.

./buildkite/ci-pipeline.yaml
  Non-authoritative YAML settings for the CI pipeline.

./buildkite/tier2-pipeline.yaml
  Non-authoritative YAML settings for the tier-2 testing (acceptance tests).

./calculate_archive_ledgers.sh
  TBD

./correct_file_names
  TBD

./download-mina-blocks
  Downloads Mina blocks stored in the Granola cloud object storage bucket by
  using `rclone`. Requires credentials (`LINODE_OBJ_ACCESS_KEY` etc.). See the
  instructions at the top of the script.

./download-staking-ledgers
  Downloads Mina staking ledgers stored in the Granola cloud object storage
  bucket by using `rclone`. Requires credentials (`LINODE_OBJ_ACCESS_KEY`
  etc.). See the instructions at the top of the script.

./granola-rclone
  Utility to wrap `rclone`. (See https://rclone.org)

./hashes.list
  TBD

./ingest-all
  Script to launch the mina-indexer and ingest all of the blocks found in
  `/mnt/mina-logs/mina_network_block_data`.

./maybe-dups
  TBD

./minaexplorer
  Directory for scripts that make use of MinaExplorer.com's data.

./minaexplorer/download-staking-ledgers
  Utility to download historical staking ledgers from MinaExplorer.com's bucket.

./minaexplorer/ledgers.json
  JSON list of known staking ledgers available.

./o1-labs
  Directory for utilities that make use of O(1) Labs's data.

./o1-labs/download-mina-blocks
  Utility to download historical Mina precomputed block logs from the bucket
  maintained by O(1) Labs.

./productionize
  Utility to simulate running a production instance of Mina Indexer. Uses
  `"$HOME"/mina-indexer` to store files. See the instructions in the script.

./rclone.conf
  A mandatory config file used by `granola-rclone`.

./upload-mina-blocks
  Utility to upload precomputed block logs to Granola's object storage.
  Requires appropriate credentials.

./upload-staking-ledgers
  Utility to upload staking ledger logs to Granola's object storage. Requires
  appropriate credentials.
