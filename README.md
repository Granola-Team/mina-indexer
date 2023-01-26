# Mina Indexer

The Mina Indexer is an improvement and simplification of the existing
Archive Node. An Archive Node is created by combining a Postgres
database, a Mina Daemon, and a Mina 'archiver' process.

The deliverable of this project is a blockchain indexer that replaces
this trio of architectural elements with a system that uses the
blocks (source of truth) by the Mina Block Producer to generate an
index for which we can efficently query.

# License (See LICENSE file for full license)

Copyright 2023 Granola Systems Inc.

Free use of this software is granted under the terms of the Mozilla
Public License 2.0.
