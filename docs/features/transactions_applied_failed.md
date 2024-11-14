# Transactions

## Context

Block producers are incentivized to include transactions in their blocks by
earning fees; each transaction carries a fee. Block producers are intended to
be agnostic to the types of transactions they include in their blocks and the
transactions in the mempool at any given time are dictated by users sending
funds around or interacting with zkapps.

Every blockchain has the concept of "applied" and "failed" transactions. When a
user submits a transaction to the network, there are a myriad of checks and
computations that are done by the block producer. E.g. block producers check to
make sure the sender has sufficient funds, a valid nonce is being used, etc.
and computate adjustments to balances based on the amount sent and fees,
new ledger hashes, etc.

If all checks pass, the transaction is "applied", meaning that it fully takes
effect in the ledger state; funds are transferred, zkapp state is adjusted,
etc.

There are many reasons why a transaction can be "failed". E.g. an incorrect
nonce is used or insufficient funds are sent to create a new account, to name a
few. For a full list of failure reasons in Mina see [this](https://github.com/Granola-Team/mina-indexer/blob/main/rust/src/protocol/serialization_types/staged_ledger_diff.rs#L456).

The effect of including a "failed" transacation in a block is that the fee goes
to the coinbase receiver and the sender's nonce is incremented, no other value
is transferred and no other changes to the state take effect.

Additionally, since several blocks may be produced at any given height, we also
have the notion of _canonicity_. The "canonical" chain represents the "true"
view of changes to the blockchain state. Only blocks on the "canonical" chain
change the state in any meaningful way.

All transactions in a "canonical" block are "canonical", independent of whether
they are "applied" or "failed".

## Mina Indexer & MinaSearch

We account for _all_ transactions, canonical or non-canonical, applied or
failed, and display this status prominently to make it as easy as possible to
fully understand an account's activity.
