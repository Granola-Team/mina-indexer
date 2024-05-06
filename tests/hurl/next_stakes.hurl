POST {{url}}
```graphql
query NextStakesQuery($limit: Int, $sort_by: NextStakesSortByInput!, $query: NextStakesQueryInput!) {
  nextStakes(limit: $limit, sortBy: $sort_by, query: $query) {
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
    nextDelegationTotals {
      countDelegates
      totalDelegated
    }
  }
}
variables {
  "limit": 2,
  "sort_by": "BALANCE_DESC",
  "query": {
    "epoch": 41,
    "public_key": "B62qiburnzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzmp7r7UN6X"
  }
}
```
HTTP 200
[Asserts]
duration < 1000
jsonpath "$.data.nextStakes" count == 1
jsonpath "$.data.nextStakes[0].balance" == 2498323.273015384
jsonpath "$.data.nextStakes[0].chainId" == "5f704c"
jsonpath "$.data.nextStakes[0].delegate" == "B62qiburnzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzmp7r7UN6X"
jsonpath "$.data.nextStakes[0].epoch" == 42
jsonpath "$.data.nextStakes[0].ledgerHash" == "jxYFH645cwMMMDmDe7KnvTuKJ5Ev8zZbWtA73fDFn7Jyh8p6SwH"
jsonpath "$.data.nextStakes[0].nonce" == 0
jsonpath "$.data.nextStakes[0].pk" == "B62qiburnzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzmp7r7UN6X"
jsonpath "$.data.nextStakes[0].public_key" == "B62qiburnzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzmp7r7UN6X"
jsonpath "$.data.nextStakes[0].token" == 1
jsonpath "$.data.nextStakes[0].voting_for" == "3NK2tkzqqK5spR2sZ7tujjqPksL45M3UUrcA4WhCkeiPtnugyE2x"
jsonpath "$.data.nextStakes[0].nextDelegationTotals.countDelegates" == 1
jsonpath "$.data.nextStakes[0].nextDelegationTotals.totalDelegated" == 2498323.273015384
