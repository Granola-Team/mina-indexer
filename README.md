# Mina Indexer

[![Build Status](https://github.com/Granola-Team/mina-indexer/actions/workflows/ci.yaml/badge.svg)](https://github.com/Granola-Team/mina-indexer/actions/workflows/ci.yaml)
[![Audit Status](https://github.com/Granola-Team/mina-indexer/actions/workflows/audit.yaml/badge.svg)](https://github.com/Granola-Team/mina-indexer/actions/workflows/audit.yaml)

The Mina indexer ("indexer") is a simplified, and improved version of
the software collectively called the "[archive
node](https://github.com/MinaProtocol/mina/tree/develop/src/app/archive)"
in the Mina codebase.

The indexer replaces the archive node trio of architectural elements
(Postgres database, Mina daemon, "mina-archiver" process) with a
system that reconstitutes the network’s historical state solely from
the precomputed blocks logged from the Mina daemon.

![High Level Architecture](notes/architecture/indexer_components.png)

The indexer's primary goals are to be easier to operate and maintain
while being a superset of the data available in the archive node.

## Warning

The indexer project is in constant development and is in an alpha
state. Functionality and API definitions will be in flux and are
subject to change without notice. With that being said, happy hacking!

## Getting Started

Clone the repo

```sh
git clone git@github.com:Granola-Team/mina-indexer.git
cd mina-indexer
```

### Building with nix

Build (install [nix](#about-the-development-environment) first)

```sh
nix develop
nix build '.?submodules=1'
```

Alternatively, you can build with `cargo` inside the nix shell
(replace `mina-indexer` by `cargo run --release --bin mina-indexer --` in all following commands).

### Building the indexer in Docker

To build the indexer in docker run the following command:

```sh
docker build -t mina-indexer:latest .
docker run --rm mina-indexer --help
```

### Starting the Indexer with a Config file

```bash
Server Config Commands

Usage: mina-indexer server config --path <PATH>

Options:
  -p, --path <PATH>  
  -h, --help         Print help
```

```yaml
ledger: /home/jenr/.mina-indexer/mainnet.json
root_hash: 3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ
startup_dir: /home/jenr/.mina-indexer/startup-blocks
watch_dir: /home/jenr/.mina-indexer/watch-blocks
watch_mode: filesystem # set to 'google_cloud' to use iggy
database_dir: /home/jenr/.mina-indexer/database
log_dir: /home/jenr/.mina-indexer/logs
keep_non_canonical_blocks: true
log_level: info
log_level_stdout: trace
prune_interval: 10
canonical_update_threshold: 2

# optional: to initialize with a snapshot
snapshot_path: /home/jenr/.mina-indexer/snapshot-20230821043522.tar.zst

# optional: configuration for google cloud block receiver (iggy)
google_cloud_watch_bucket: mina_network_block_data
google_cloud_watcher_lookup_num: 20
google_cloud_watcher_lookup_freq: 30 # seconds
google_cloud_watcher_lookup_network: mainnet
```


### Server Commands

```bash
Start the mina indexer by passing in arguments manually on the command line

Usage: mina-indexer server cli [OPTIONS] --ledger <LEDGER>

Options:
  -l, --ledger <LEDGER>
          Path to the root ledger
      --root-hash <ROOT_HASH>
          Hash of the root ledger [default: 3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ]
  -s, --startup-dir <STARTUP_DIR>
          Path to startup blocks directory [default: /Users/jenr24/.mina-indexer/startup-blocks]
  -w, --watch-dir <WATCH_DIR>
          Path to directory to watch for new blocks, if using the filesystem block receiver [default: /Users/jenr24/.mina-indexer/watch-blocks]
      --google-cloud-watch-bucket <GOOGLE_CLOUD_WATCH_BUCKET>
          Google Cloud bucket to watch for new blocks, if using the Google Cloud Block Receiver (iggy) [default: mina_network_block_data]
      --google-cloud-watcher-lookup-num <GOOGLE_CLOUD_WATCHER_LOOKUP_NUM>
          number of blocks to lookup on each query of the Google Cloud Block Receiver
      --google-cloud-watcher-lookup-freq <GOOGLE_CLOUD_WATCHER_LOOKUP_FREQ>
          Frequency to lookup blocks with iggy (seconds)
      --google-cloud-watcher-lookup-network <GOOGLE_CLOUD_WATCHER_LOOKUP_NETWORK>
          Mina Network to query when using iggy [possible values: mainnet, berkeley, testnet]
      --watch-mode <WATCH_MODE>
          Which mode to receive new blocks in (goole cloud or filesystem) [default: filesystem] [possible values: filesystem, google-cloud]
  -d, --database-dir <DATABASE_DIR>
          Path to directory for rocksdb [default: /Users/jenr24/.mina-indexer/database]
      --log-dir <LOG_DIR>
          Path to directory for logs [default: /Users/jenr24/.mina-indexer/logs]
  -k, --keep-non-canonical-blocks
          Only store canonical blocks in the db
      --log-level <LOG_LEVEL>
          Max file log level [default: debug]
      --log-level-stdout <LOG_LEVEL_STDOUT>
          Max stdout log level [default: info]
  -p, --prune-interval <PRUNE_INTERVAL>
          Interval for pruning the root branch [default: 10]
  -c, --canonical-update-threshold <CANONICAL_UPDATE_THRESHOLD>
          Threshold for updating the canonical tip/ledger [default: 2]
      --snapshot-path <SNAPSHOT_PATH>
          Path to an indexer snapshot
  -h, --help
          Print help
  -V, --version
          Print version
```

### Client Commands

```bash
Client commands

Usage: mina-indexer client <COMMAND>

Commands:
  account      Display the account info for the given public key
  best-chain   Display the best chain
  best-ledger  Dump the best ledger to a file
  summary      Show summary of indexer state
  help         Print this message or the help of the given subcommand(s)

Options:
  -o, --output-json Output JSON data when possible
  -h, --help        Print help
  -V, --version     Print version
```

### Some useful `client` commands

Query data with the `mina-indexer` client (from another terminal window)

* Get the account info for a specific Public Key
```sh
mina-indexer client account --public-key $PUBLIC_KEY
```

* Get the current best chain of blocks from the tip, length `NUM`
```sh
mina-indexer client best-chain --num $NUM
```

* Dump the best ledger to a file
```sh
mina-indexer client best-ledger --path $PATH
```

* Get a summary of the indexer state
```sh
mina-indexer client summary
```

* Get a verbose summary of the indexer state (pretty pictures included!)
```sh
mina-indexer client summary -v
```

### Help

For more information, check out the help menus

```sh
mina-indexer server --help
mina-indexer client --help
```

## About the development environment

This repository uses Nix Flakes as a development environment and build system. You can install Nix [here](https://nixos.org/download.html) Sand you can visit [this page](https://nixos.wiki/wiki/Flakes) for instructions on enabling Nix Flakes on your system. Apart from Nix, there are no external dependencies for this project!

## Building the Project

Binaries for `mina-indexer` can be built by running `nix build '.?submodules=1'` with Flakes enabled (see above). All binaries are output to `./result/bin`

## Entering a Development Environment

You can enter a development environment by running `nix develop` at the command line. The development environment for this project takes care of installing all dependencies, compilers, and development tools (this means that you don't even need rustup installed!), including the `rust-analyzer` language server. For VSCode, we recommend the `Nix Environment Selector` extension pointed at `shell.nix` to tell your IDE about the installed tools, though you can also use direnv for this same purpose.

## Running unit tests

In the nix shell issue the following command to run the unit tests.

`cargo nextest run`

## License (See LICENSE file for full license)

Copyright 2022-2023 Mina Foundation, Inc.

Free use of this software is granted under the terms of the Mozilla
Public License 2.0.
