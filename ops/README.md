# ops

## Operations Utilities

Files in this directory:

- `./calculate-archive-ledgers.sh`
- `./check-contiguous-blocks.rb`
- `./correct-file-names.sh`
- `./deploy.rb`
- `./download-mina-blocks.rb`
- `./download-staking-ledgers.rb`
- `./format-graphql-in-hurl-files.rb`
- `./granola-rclone.rb`
- `./hashes.list`
- `./indexer-ledger-normalizer.rb`
- `./maybe-dupes.list`
- `./mina-ledger-normalizer.rb`
- `./minaexplorer/download-staking-ledgers.sh`
- `./minaexplorer/ledgers.json`
- `./o1-labs/download-mina-blocks.sh`
- `./rclone.conf`
- `./recycle-pcbs.rb`
- `./stage-blocks.rb`
- `./traverse-canonical-chain.sh`
- `./unformat-pcbs.rb`
- `./update-pcbs.rb`
- `./upload-mina-blocks.sh`
- `./upload-staking-ledgers.sh`
- `./utils.rb`

### `./calculate-archive-ledgers.sh`

TBD

### `./correct-file-names.sh`

TBD

### `./check-contiguous-blocks.rb`

Scans a directory of Mina PCB JSON files and checks for contiguous block numbers in the filenames.

### `./deploy.rb`

Utility to manage the running of a production instance of Mina Indexer. See the instructions in the script.

### `./diff-buckets.rb`

Compare the list of historical Mina PCBs in the specified block range between o1Labs and Granola storage buckets.

### `./download-mina-blocks.rb`

Downloads historical Mina PCBs in the Granola storage bucket by using `rclone`. See the instructions at the top of the script.

### `./download-staking-ledgers.rb`

Downloads Mina staking ledgers stored in the Granola storage bucket by using `rclone`. See the instructions at the top of the script.

### `./format-graphql-in-hurl-files.rb`

Standardize formatting of GraphQL in `hurl` files.

### `./granola-rclone.rb`

Utility to wrap `rclone`. See [rclone](https://rclone.org).

### `./hashes.list`

TBD

### `/.indexer-ledger-normalizer.rb`

TBD

### `./maybe-dupes.list`

TBD

### `./mina/mina_txn_hasher.exe`

Computes Berkeley hashes for different types of commands (e.g. user or zkApp).

### `./minaexplorer/download-staking-ledgers.sh`

Utility to download historical staking ledgers from MinaExplorer.com's Google Cloud Storage bucket.

### `./minaexplorer/ledgers.json`

JSON list of known staking ledgers available. See [MinaExplorer docs](https://docs.minaexplorer.com/minaexplorer/data-archive).

### `./o1-labs/download-mina-blocks.rb`

Downloads historical Mina PCBs from o1Labs' [Google Cloud Storage bucket](https://storage.googleapis.com/storage/v1/b/mina_network_block_data/o?prefix=mainnet-).

### `./rclone.conf`

A mandatory config file used by `granola-rclone.rb`.

### `./recycle-pcbs.rb`

TBD

### `./stage-block.sh`

TBD

### `./traverse-canonical-chain.sh`

TBD

### `./unformat-pcbs.rb`

Compact JSON into single line

### `./update-pcbs.rb`

Remove any "proofs" array from the PCB JSON files in a directory. This significantly shrinks files and helps ensure no invalid unicde characters exist. In addition, it adds a v2 hash for any transaction to the PCB.

### `./upload-mina-blocks.sh`

Utility to upload PCBs to Granola's storage bucket. Requires appropriate credentials.

### `./upload-staking-ledgers.sh`

Utility to upload staking ledger logs to Granola's storage bucket. Requires appropriate credentials.

### `./utils.rb`

TBD
