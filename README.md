# mina-indexer

## Getting Started

### Prerequisites

- [Install Flox](https://flox.dev/docs/install-flox/)
  - This will install Nix for you
  - If you have previously installed Nix using [The Determinate Nix Installer](https://github.com/DeterminateSystems/nix-installer#the-determinate-nix-installer), you may need to use [the uninstaller](https://github.com/DeterminateSystems/nix-installer#uninstalling) before installing Flox.

### Development

You must first enter the Flox environment by using `flox activate`.

#### Format

`cargo-fmt --all`

#### Check

`cargo-fmt --all --check && cargo clippy --all-targets --all-features -- -D warnings`

#### Build

`cargo build`

#### Run

`cargo run --bin ingest_staking_ledgers`
