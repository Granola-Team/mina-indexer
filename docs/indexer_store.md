# Indexer Store

## User Command Store

User commands have a corresponding transaction hash `txn_hash`. Only _one_ user command with a given `txn_hash` can appear in any given block. However, a command can appear in multiple blocks in general. Thus, we cannot uniquely identify a user command _only_ by its `txn_hash`, we choose to use both `txn_hash` and `state_hash` of the containing block.

User commands are stored in various ways:

- for storage of all user commands
  - key: `{txn_hash}{state_hash}`
  - val: signed command with data

- for storage of all user commands per block
  - key: `{state_hash}`
  - val: list of user commands with status

- for sorting all user commands sorting by global slot
  - key: `{slot}{txn_hash}{state_hash}`
  - val: _empty_

  where `slot` is in big endian so lexicographic ordering of keys corresponds to global slot ordering. Hence, iteration by global slot is just a simple database CF iterator!

- for sorting by sender/receiver & global slot
  - key: `{pk}{slot}{txn_hash}{state_hash}`
  - val: _empty_

- for querying user commands by block
  - key: `{state_hash}{index}`
  - val: `index`th txn in block with `state_hash`

- for querying user commands by `txn_hash`
  - key: `{txn_hash}{index}`
  - val: user command with `txn_hash` in `index`th best containing block
