# The Mina Indexer

Constructs and serves indices of the Mina blockchain via GraphQL.

![Build Badge](https://badge.buildkite.com/c2da30c5a1deb1ff6e0ca09c5ec33f7bd0a5b57ea35df4fc15.svg)
![LICENCE Badge](https://img.shields.io/badge/license-APACHE-blue.svg)

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
rake test
```

### Regression Tests

To quickly perform regression tests, which check for new bugs in existing
features after updates, use:

```bash
rake dev
```

To perform the test battery that the CI runs, use:

```bash
rake test
rake test_system
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
