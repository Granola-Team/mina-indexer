POST {{url}}
```graphql
query StakesQuery($limit: Int, $sort_by: StakeSortByInput!, $query: StakeQueryInput!) {
  stakes(limit: $limit, sortBy: $sort_by, query: $query) {
    balance
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
duration < 1000
jsonpath "$.data.stakes" count == 1
jsonpath "$.data.stakes[0].balance" == 2498323.273015384
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

POST {{url}}
```graphql
query StakesQuery($limit: Int, $sort_by: StakeSortByInput!, $query: StakeQueryInput!) {
  stakes(limit: $limit, sortBy: $sort_by, query: $query) {
    pk
    balance
    delegate
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
    "ledgerHash": "jx7buQVWFLsXTtzRgSxbYcT8EYLS8KCZbLrfDcJxMtyy4thw2Ee",
    "delegate": "B62qrxNgwAdhGYZv1BXQRt2HgopUceFyrtXZMikwsuaHu5FigRJjhwY"
  }
}
```
HTTP 200
[Asserts]
duration < 1000
jsonpath "$.data.stakes" count == 2
jsonpath "$.data.stakes[0].timing.cliff_amount" == 1000000000
jsonpath "$.data.stakes[0].timing.cliff_time" == 345600
jsonpath "$.data.stakes[0].timing.initial_minimum_balance" == 1000000000
jsonpath "$.data.stakes[0].timing.vesting_increment" == 0
jsonpath "$.data.stakes[0].timing.vesting_period" == 1
