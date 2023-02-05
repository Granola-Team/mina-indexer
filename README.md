# Mina Indexer

The Mina indexer is an improvement and simplification over the
software that is collectively called "[archive
node](https://github.com/MinaProtocol/mina/tree/develop/src/app/archive)"
in the Mina codebase.

The Mina indexer replaces the archive node trio of architectural
elements (PostgreSQL database, Mina daemon, 'mina-archiver' process)
with a system that consumes the precomputed blocks by the Mina daemon
to generate an index for which we can efficiently query.

# License (See LICENSE file for full license)

Copyright 2023 Granola Systems Inc.

Free use of this software is granted under the terms of the Mozilla
Public License 2.0.
