# User Commands (Transactions)

## What are User Commands?

User commands, commonly called transactions, are user-initiated
actions on the Mina blockchain. There are two types of transactions:
payment transactions and stake delegations.

* **Payment Transactions:** Users send MINA to other users.
* **Stake Delegations:** Users delegate their staking weight to
  validators (block producers) to participate in consensus and earn
  rewards.

## Failed Transactions

User transactions can and do fail, and these scenarios must be managed
accordingly. If a payment transaction fails, the sender will not be
charged the transaction amount but will still be required to pay the
fee to the SNARK worker. Similarly, if a stake delegation fails, the
fee will be charged, but there is no payment to return.

# Account Creation Fee

Transactions directed to accounts that have not previously received
transactions will incur an account creation fee of 1 MINA. This
applies to both user transactions and coinbase transactions.

# Internal Commands

## What are Internal Commands?

Internal commands are actions executed by the protocol itself rather
than being initiated by users. These actions include awarding coinbase
rewards and initiating fee transfers to compensate for SNARK work.

## Fee Transfer

Fee transfers are used to pay prover fees to SNARK workers. A
corresponding proof must be included for every transaction occupying a
slot. Fee transfers can originate from transaction fees or the
coinbase, with the latter being referred to as
`fee_transfer_via_coinbase`.

If the transaction fees exceed the total SNARK work fees, the
remainder will be sent to the block producer via a fee
transfer. Likewise, if the transaction fees don't cover the SNARK
work, the deficit will be paid from the coinbase reward.

## Coinbase Transaction

These are protocol-initiated rewards given to validators for producing
a block. Similar to a user transaction, there may be a fee associated
with the transaction to cover SNARK work. In cases where there is no
SNARK fee, the full coinbase reward will be awarded to the block
producer. If the SNARK worker does charge a fee, this fee will be
deducted from the coinbase reward, and the deduction will be sent to
the SNARK worker through a `fee_transfer_via_coinbase` transfer.

There is a peculiar implementation detail where, at most, 2 coinbase
transactions may be sent, which also implies that potentially, at
most, 2 `fee_transfer_via_coinbase` transfers may occur. The coinbase
reward, minus any fees, is divided equally across these two coinbase
transactions.

## Fee Transfer via Coinbase

Fee transfer via coinbase operates much like fee transfer, except the
fee comes from the coinbase.

### Preconditions for a Fee Transfer via Coinbase

* When there are no transactions, meaning no transaction fees are
  available to pay for the SNARK work needed to include the coinbase
  transaction.

* This includes scenarios where transactions cannot be included due to
  high SNARK work fees.

* When the SNARK work fees exceed the transaction fees.

# Reading Material

* [Mina Explorer BigQuery Public Dataset](https://docs.minaexplorer.com/minaexplorer/bigquery-public-dataset)
* [Mina Protocol GitHub](https://github.com/MinaProtocol/mina/blob/compatible/src/app/replayer/replayer.ml)
