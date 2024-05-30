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

### Prerequisites

1. Install Nix [here](https://nixos.org/download.html).
2. Enable Flakes using [this guide](https://nixos.wiki/wiki/Flakes).
3. Install [Direnv](https://direnv.net).

### Building the Project

Run `just build` to compile the project. This will compile the `mina-indexer`
binary. See the `./scripts` folder for examples.

### Running Tests

#### Unit Tests

Execute unit tests to validate code functionality with:

```bash
just test-unit
```

#### Regression Tests

To perform regression tests, which check for new bugs in existing
features after updates, use:

```bash
just test-regression
```

## Generating OCI Images With Nix

Building OCI (Docker) images from Nix must happen from `x86-64-linux` machine
otherwise you'll need to cross-compile.

Issue the following command to build the image and load it into Docker.

```bash
just build-images
```

## License

Copyright 2022-2024 Granola Systems Inc.

This software is [licensed](LICENSE) under the Apache License, Version 2.0.

## Contributing

This project uses [C4(Collective Code Construction
Contract)](https://rfc.zeromq.org/spec/42/) process for contributions.
