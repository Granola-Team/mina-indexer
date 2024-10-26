# Mina Indexer

[![Build
status](https://badge.buildkite.com/c2da30c5a1deb1ff6e0ca09c5ec33f7bd0a5b57ea35df4fc15.svg)](https://buildkite.com/granola/mina-indexer)

The Mina Indexer is a redesigned version of the software collectively
called the "[Mina archive
node](https://github.com/MinaProtocol/mina/tree/develop/src/app/archive)."

**Note:** As the project is in active development, be aware that the
public APIs and functionalities are subject to change.

## Motivation

Working with the Mina Archive Node can be complex and
time-consuming. It requires the exact configuration of multiple
components — a Mina Node, an Archive Node, and a Postgres database —
and an in-depth knowledge of SQL and the Mina blockchain.

Additionally, even with a proper setup, the Archive Node system is
prone to missing blocks, creating gaps in the data. These gaps require
manual intervention for resolution, which adds layers of complexity to
the system's management.

A major problem with the Archive Node is its reliance on a `pg_dump`
from a currently active node for initial setup. This approach
centralizes data, necessitating trust from the operator's side.

## Solution

The Mina Indexer addresses this by simplifying the initial
configuration by using precomputed blocks as the source of truth,
bypassing the need for external database dumps.

We designed the Mina Indexer to be a comprehensive replacement for the
Mina Archive Node, providing an easier-to-use platform with native
support for the [Rosetta
API](https://www.rosetta-api.org/docs/welcome.html). We aim to
streamline blockchain interaction for developers within the Mina
ecosystem by providing developers and operators with a better toolset
optimized for the Mina ecosystem.

## Getting Started

### Development Prerequisites

1. Install Nix (with Flakes) using [The Determinate Nix Installer](https://github.com/DeterminateSystems/nix-installer#the-determinate-nix-installer).
2. Install and configure [Direnv](https://direnv.net).

### Execution Environment

Invoking `mina-indexer database create` on very large blocks directories will
ingest the blocks at very high speed. This requires setting `ulimit -n` to a
larger-than-default value (for many Linux distros). 1024 is not sufficient. See
your distro's documentation on how to set `ulimit -n` (max open files) to 4096
or more.

Normal operation of the 'mina-indexer' does not require this change.

### Building the Project

Run `just check` to verify that the project compiles. This will compile the
`mina-indexer` binary in debug mode.

### Storage

The default storage location is on `/mnt` because the testing code may download
large volumes of test data, and placing on `/mnt` gives an opportunity to use
different storage volumes from one's build directory.

Set the `VOLUMES_DIR` environment variable if you want to replace `/mnt` with
another path.

### Testing

#### Unit Tests

Execute [unit tests](/rust/tests) to validate code functionality with:

```bash
just test-unit
```

#### Regression Tests

To invoke the [regression test suite](/tests/regression.bash), the directory `/mnt/mina-indexer-dev` must
exist.

To quickly perform regression tests, which check for new bugs in existing
features after updates, use:

```bash
just dev
```

To perform the test battery that the [(tier-1) CI](https://buildkite.com/granola/mina-indexer-tier-1) runs, use:

```bash
just tier1
```

#### More Tests

To invoke a more comprehensive regression test suite, the directory
`/mnt/mina-indexer-test` must exist.

Invoke:

```bash
just tier2
```

Or, for even more testing:

```bash
just tier3
```

### Deployment

To deploy a mina-indexer locally, the directory `/mnt/mina-indexer-prod`
must exist.

There are two options to start an instance:
1. `just deploy-local-prod` uses the release binary
2. `just deploy-local-prod-dev` uses the debug binary

## Generating OCI Images With Nix

Note: This requires [the Docker Engine](https://docs.docker.com/engine/install/) to be installed.

Building the OCI (Docker) image from Nix must happen from an `x86-64-linux`
machine.

Issue the following command to build the image and load it into Docker:

```bash
just build-image
```

## License

Copyright 2022-2024 Granola Systems Inc.

This software is [licensed](LICENSE) under the Apache License, Version 2.0.

## Contributing

This project uses [C4(Collective Code Construction
Contract)](https://rfc.zeromq.org/spec/42/) process for contributions.
Additionally, maintainers are permitted to push directly to 'main'.
