# Ledger Calculation Invariants

* **Genesis Ledger Accounts:** Accounts in the genesis ledger are
  exempt from the `1.0` Mina account creation fee. They can be found
  [here](../rust/data/genesis_blocks/mainnet-1-3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ.json).

* **Genesis Block Winner:** The genesis block winner's account is
  credited with `0.000001000` Mina by the node. This account doesn't
  appear in the genesis ledger but isn't subject to account creation
  fees.

* **Transaction Nonce:** Only the sender of a transaction included in
  the canonical chain has their nonce incremented, regardless of the
  transaction's success or failure.
