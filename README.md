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

This project utilizes Nix Flakes for development and building. Install
Nix [here](https://nixos.org/download.html) and enable Flakes using
the guide [here](https://nixos.wiki/wiki/Flakes). No additional
dependencies are needed.

### Development Setup

Use `nix develop` to prepare your development environment. It
configures your current shell with all necessary dependencies and
tools, removing the need for a separate host Rust installation.

### Building the Project

Run `nix build` to compile the project. This will
compile the `mina-indexer` binary and place it in `./result/bin`.

### Running Tests

#### Unit Tests

Execute unit tests to validate code functionality with:

```bash
nix-shell --run "just test-unit"
```

#### Regression Tests

To perform regression tests, which check for new bugs in existing
features after updates, use:

```bash
nix-shell --run "just test-regression"
```

## License (See LICENSE file for full license)

Copyright 2022-2024 Mina Foundation, Inc.

Free use of this software is granted under the terms of the Mozilla
Public License 2.0.

## Contributing

This project uses [C4(Collective Code Construction
Contract)](https://rfc.zeromq.org/spec/42/) process for contributions.
