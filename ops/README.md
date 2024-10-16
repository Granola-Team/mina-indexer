# ops

## Operations Utilities

Files in this directory:

```
./calculate-archive-ledgers.sh
./correct-file-names.sh
./deploy.rb
./download-mina-blocks.rb
./download-staking-ledgers.rb
./granola-rclone.rb
./hashes.list
./indexer-ledger-normalizer.rb
./maybe-dupes.list
./mina-ledger-normalizer.rb
./minaexplorer/download-staking-ledgers.sh
./minaexplorer/ledgers.json
./o1-labs/download-mina-blocks.sh
./rclone.conf
./recycle-pcbs.rb
./stage-blocks.rb
./traverse-canonical-chain.sh
./unformat-pcbs.sh
./upload-mina-blocks.sh
./upload-staking-ledgers.sh
```

./calculate-archive-ledgers.sh
  TBD

./correct-file-names.sh
  TBD

./deploy.rb
  Utility to manage the running of a production instance of Mina Indexer. See
  the instructions in the script.

./download-mina-blocks.rb
  Downloads Mina blocks stored in the Granola cloud object storage bucket by
  using `rclone`. Requires credentials (`LINODE_OBJ_ACCESS_KEY` etc.). See the
  instructions at the top of the script.

./download-staking-ledgers.rb
  Downloads Mina staking ledgers stored in the Granola cloud object storage
  bucket by using `rclone`. Requires credentials (`LINODE_OBJ_ACCESS_KEY`
  etc.). See the instructions at the top of the script.

./granola-rclone.rb
  Utility to wrap `rclone`. (See https://rclone.org)

./hashes.list
  TBD

/.indexer-ledger-normalizer.rb
  TBD

./maybe-dupes.list
  TBD

./minaexplorer
  Directory for scripts that make use of MinaExplorer.com's data.

./minaexplorer/download-staking-ledgers.sh
  Utility to download historical staking ledgers from MinaExplorer.com's bucket.

./minaexplorer/ledgers.json
  JSON list of known staking ledgers available.

./o1-labs
  Directory for utilities that make use of O1Labs's data.

./o1-labs/download-mina-blocks.sh
  Utility to download historical Mina precomputed block logs from the bucket
  maintained by O1Labs.

./rclone.conf
  A mandatory config file used by `granola-rclone.rb`.

./recycle-pcbs.rb
  TBD

./stage-block.sh
  TBD

./traverse-canonical-chain.sh
  TBD

./unformat-pcbs.sh
  TBD

./upload-mina-blocks.sh
  Utility to upload precomputed block logs to Granola's object storage.
  Requires appropriate credentials.

./upload-staking-ledgers.sh
  Utility to upload staking ledger logs to Granola's object storage. Requires
  appropriate credentials.
