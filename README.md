# The Mina Indexer

<a href="https://buildkite.com/granola/mina-indexer-tier-1/builds?branch=main"><img src="https://badge.buildkite.com/c2da30c5a1deb1ff6e0ca09c5ec33f7bd0a5b57ea35df4fc15.svg"></a>
<a href="https://github.com/Granola-Team/mina-indexer/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-APACHE-blue.svg"></a>

Constructs and serves indices of the Mina blockchain.

The Mina Indexer uses precomputed blocks (logged by a Mina node) as the source
of truth for the blockchain.

See an instance of the Mina Indexer in action at [MinaSearch](https://minasearch.com).

## Development Prerequisites

1. Install [Nix](https://determinate.systems/nix-installer/).
2. Install and configure [Direnv](https://direnv.net).

## Execution Environment

Set `ulimit -n` (max open files) to 4096 or more.

## Building the Project

Run `rake check` to check for errors. See also the output of `rake` for other
options.

## Storage

The default storage location is on `/mnt` because the testing code may download
large volumes of test data, and placing on `/mnt` gives an opportunity to use
different storage volumes from one's build directory.

Set the `VOLUMES_DIR` environment variable if you want to replace `/mnt` with
another path.

## Testing

### Unit Tests

Execute [unit tests](/rust/tests) to validate code functionality with:

```bash
rake test:unit:tier1
rake test:unit:tier2
```

### Regression Tests

To quickly perform regression tests, which check for new bugs in existing
features after updates, use:

```bash
rake dev
```

To perform the test battery that the [(tier-1) CI](https://buildkite.com/granola/mina-indexer-tier-1) runs, use:

```bash
rake tier1
```

## Deployment

`rake deploy:local_prod` uses the Nix-based release binary

## Generating OCI Images With Nix

Note: This requires [the Docker Engine](https://docs.docker.com/engine/install/) to be installed.

Building the OCI (Docker) image from Nix must happen from an `x86-64-linux`
machine.

Issue the following command to build the image and load it into Docker:

```bash
rake build:oci_image
```

## License

Copyright 2022-2025 Granola Systems Inc.

This software is [licensed](LICENSE) under the Apache License, Version 2.0.
