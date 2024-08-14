#
# Stakes epoch public query
#

POST {{url}}
```graphql
query StakesQuery($limit: Int, $sort_by: StakeSortByInput!, $query: StakeQueryInput!) {
  stakes(limit: $limit, sortBy: $sort_by, query: $query) {
    balance
    balanceNanomina
    chainId
    delegate
    epoch
    ledgerHash
    nonce
    pk
    public_key
    token
    voting_for
    delegationTotals {
      countDelegates
      totalDelegated
      delegates
    }
  }
}

variables {
  "limit": 2,
  "sort_by": "BALANCE_DESC",
  "query": {
    "epoch": 42,
    "public_key": "B62qiburnzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzmp7r7UN6X"
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.stakes" count == 1

# only datum
jsonpath "$.data.stakes[0].balance" == 2498323.273015384
jsonpath "$.data.stakes[0].balanceNanomina" == 2498323273015384
jsonpath "$.data.stakes[0].chainId" == "5f704c"
jsonpath "$.data.stakes[0].delegate" == "B62qiburnzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzmp7r7UN6X"
jsonpath "$.data.stakes[0].epoch" == 42
jsonpath "$.data.stakes[0].ledgerHash" == "jxYFH645cwMMMDmDe7KnvTuKJ5Ev8zZbWtA73fDFn7Jyh8p6SwH"
jsonpath "$.data.stakes[0].nonce" == 0
jsonpath "$.data.stakes[0].pk" == "B62qiburnzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzmp7r7UN6X"
jsonpath "$.data.stakes[0].public_key" == "B62qiburnzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzmp7r7UN6X"
jsonpath "$.data.stakes[0].token" == 1
jsonpath "$.data.stakes[0].voting_for" == "3NK2tkzqqK5spR2sZ7tujjqPksL45M3UUrcA4WhCkeiPtnugyE2x"
jsonpath "$.data.stakes[0].delegationTotals.countDelegates" == 1
jsonpath "$.data.stakes[0].delegationTotals.totalDelegated" == 2498323.273015384

jsonpath "$.data.stakes[0].delegationTotals.delegates" count == 1
jsonpath "$.data.stakes[0].delegationTotals.delegates[0]" == "B62qiburnzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzmp7r7UN6X"

duration < 12000

#
# Stakes ledger hash delegate timing info query
#

POST {{url}}
```graphql
query StakesQuery($limit: Int, $sort_by: StakeSortByInput!, $query: StakeQueryInput!) {
  stakes(limit: $limit, sortBy: $sort_by, query: $query) {
    timing {
      cliff_amount
      cliff_time
      initial_minimum_balance
      vesting_increment
      vesting_period
    }
  }
}

variables {
  "limit": 2,
  "sort_by": "BALANCE_DESC",
  "query": {
    "ledgerHash": "jxYFH645cwMMMDmDe7KnvTuKJ5Ev8zZbWtA73fDFn7Jyh8p6SwH",
    "delegate": "B62qrxNgwAdhGYZv1BXQRt2HgopUceFyrtXZMikwsuaHu5FigRJjhwY"
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.stakes" count == 2

# first datum
jsonpath "$.data.stakes[0].timing.cliff_amount" == 1000000000
jsonpath "$.data.stakes[0].timing.cliff_time" == 345600
jsonpath "$.data.stakes[0].timing.initial_minimum_balance" == 1000000000
jsonpath "$.data.stakes[0].timing.vesting_increment" == 0
jsonpath "$.data.stakes[0].timing.vesting_period" == 1

# last datum
jsonpath "$.data.stakes[1].timing" == null

duration < 10000

#
# Stakes delegation total stake percentage query
#

POST {{url}}
```graphql
{
  stakes(query: {epoch: 42, public_key: "B62qptmpH9PVe76ZEfS1NWVV27XjZJEJyr8mWZFjfohxppmS11DfKFG"} limit: 1) {
    delegationTotals {
      totalStakePercentage
    }
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.stakes" count == 1

# only datum
jsonpath "$.data.stakes[0].delegationTotals.totalStakePercentage" == "8.56"

duration < 5000

#
# Stakes pk & total/epoch counts
#

POST {{url}}
```graphql
{
  stakes(query: {epoch: 0, public_key: "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy"}, limit: 1) {
    pk_epoch_num_blocks
    pk_total_num_blocks
    pk_epoch_num_snarks
    pk_total_num_snarks
    pk_epoch_num_user_commands
    pk_total_num_user_commands
    pk_epoch_num_internal_commands
    pk_total_num_internal_commands
    epoch_num_blocks
    total_num_blocks
    epoch_num_snarks
    total_num_snarks
    epoch_num_user_commands
    total_num_user_commands
    epoch_num_internal_commands
    total_num_internal_commands
  }
}
```
HTTP 200
[Asserts]

jsonpath "$.data.stakes" count == 1

# only datum
jsonpath "$.data.stakes[0].pk_epoch_num_blocks" == 0
jsonpath "$.data.stakes[0].pk_total_num_blocks" == 0
jsonpath "$.data.stakes[0].pk_epoch_num_snarks" == 0
jsonpath "$.data.stakes[0].pk_total_num_snarks" == 0
jsonpath "$.data.stakes[0].pk_epoch_num_user_commands" == 294
jsonpath "$.data.stakes[0].pk_total_num_user_commands" == 294
jsonpath "$.data.stakes[0].pk_epoch_num_internal_commands" == 196
jsonpath "$.data.stakes[0].pk_total_num_internal_commands" == 196
jsonpath "$.data.stakes[0].epoch_num_blocks" == 204
jsonpath "$.data.stakes[0].total_num_blocks" == 204
jsonpath "$.data.stakes[0].epoch_num_snarks" == 64
jsonpath "$.data.stakes[0].total_num_snarks" == 64
jsonpath "$.data.stakes[0].epoch_num_user_commands" == 303
jsonpath "$.data.stakes[0].total_num_user_commands" == 303
jsonpath "$.data.stakes[0].epoch_num_internal_commands" == 205
jsonpath "$.data.stakes[0].total_num_internal_commands" == 205

duration < 10000

#
# Staking ledger epoch query
#

POST {{url}}
```graphql
{
  stakes(query: {ledgerHash: "jxYFH645cwMMMDmDe7KnvTuKJ5Ev8zZbWtA73fDFn7Jyh8p6SwH"}, limit: 0) {
    epoch
  }
}
```
HTTP 200
[Asserts]

jsonpath "$.data.stakes" count == 1

# only datum
jsonpath "$.data.stakes[0].epoch" == 42

duration < 200

#
# Stakes ledger hash (default stake descending) query
#

POST {{url}}
```graphql
query StakesQuery($limit: Int, $query: StakeQueryInput!) {
  stakes(limit: $limit, query: $query) {
    pk
    balance
    balanceNanomina
    username
  }
}

variables {
  "limit": 10,
  "query": {
    "ledgerHash": "jxYFH645cwMMMDmDe7KnvTuKJ5Ev8zZbWtA73fDFn7Jyh8p6SwH"
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.stakes" count == 10

# first datum
jsonpath "$.data.stakes[0].pk" == "B62qptmpH9PVe76ZEfS1NWVV27XjZJEJyr8mWZFjfohxppmS11DfKFG"
jsonpath "$.data.stakes[0].balance" == 83482371.13484836
jsonpath "$.data.stakes[0].balanceNanomina" == 83482371134848360
jsonpath "$.data.stakes[0].username" == "Unknown"

# last datum
jsonpath "$.data.stakes[9].pk" == "B62qq3TQ8AP7MFYPVtMx5tZGF3kWLJukfwG1A1RGvaBW1jfTPTkDBW6"
jsonpath "$.data.stakes[9].balance" == 1104.686899999
jsonpath "$.data.stakes[9].balanceNanomina" == 1104686899999
jsonpath "$.data.stakes[9].username" == "Unknown"

duration < 10000

#
# Stakes delegation total stake percentage query sort by stakes desc
#

POST {{url}}
```graphql
{
  stakes(query: {epoch: 42}, sortBy: STAKE_DESC, limit: 5) {
    delegationTotals {
      totalStakePercentage
    }
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.stakes" count == 5

# only datum
jsonpath "$.data.stakes[0].delegationTotals.totalStakePercentage" == "8.56"
jsonpath "$.data.stakes[1].delegationTotals.totalStakePercentage" == "5.06"
jsonpath "$.data.stakes[2].delegationTotals.totalStakePercentage" == "3.79"
jsonpath "$.data.stakes[3].delegationTotals.totalStakePercentage" == "3.50"
jsonpath "$.data.stakes[4].delegationTotals.totalStakePercentage" == "3.10"

duration < 10000

# Test for public key uniqueness per epoch
POST {{url}}
```graphql
{
  stakes(query: {epoch: 42, public_key: "B62qpge4uMq4Vv5Rvc8Gw9qSquUYd6xoW1pz7HQkMSHm6h1o7pvLPAN"}) {
    public_key
    balance
    nonce
  }
}
```
HTTP 200
[Asserts]

jsonpath "$.data.stakes" count == 1
jsonpath "$.data.stakes[0].public_key" == "B62qpge4uMq4Vv5Rvc8Gw9qSquUYd6xoW1pz7HQkMSHm6h1o7pvLPAN"
jsonpath "$.data.stakes[0].balance" == 92.929966438
jsonpath "$.data.stakes[0].nonce" == 4

duration < 10000

#
# Test for existance of epoch 0 staking ledger in the presence of epoch 1 staking ledger
#

POST {{url}}
```graphql
{
  stakes(query: {epoch: 0, ledgerHash: "jx7buQVWFLsXTtzRgSxbYcT8EYLS8KCZbLrfDcJxMtyy4thw2Ee"}, limit: 1) {
    epoch
  }
}
```
HTTP 200
[Asserts]

jsonpath "$.data.stakes" count == 1
jsonpath "$.data.stakes[0].epoch" == 0

duration < 1000

#
# Test for existance of epoch 1 staking ledger in the presence of epoch 0 staking ledger
#

POST {{url}}
```graphql
{
  stakes(query: {epoch: 1, ledgerHash: "jx7buQVWFLsXTtzRgSxbYcT8EYLS8KCZbLrfDcJxMtyy4thw2Ee"}, limit: 1) {
    epoch
    epoch_num_accounts
  }
}
```
HTTP 200
[Asserts]

jsonpath "$.data.stakes" count == 1
jsonpath "$.data.stakes[0].epoch" == 1
jsonpath "$.data.stakes[0].epoch_num_accounts" == 1676

duration < 1000
