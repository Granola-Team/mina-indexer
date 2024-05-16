#
# Next stakes epoch public key query
#

POST {{url}}
```graphql
query NextStakesQuery($limit: Int, $sort_by: NextstakeSortByInput!, $query: NextstakeQueryInput!) {
  nextstakes(limit: $limit, sortBy: $sort_by, query: $query) {
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
    pk_epoch_num_blocks
    pk_total_num_blocks
    epoch_num_blocks
    total_num_blocks
  }
}

variables {
  "limit": 1,
  "sort_by": "BALANCE_DESC",
  "query": {
    "epoch": 41,
    "public_key": "B62qiburnzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzmp7r7UN6X"
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.nextstakes" count == 1

# only datum
jsonpath "$.data.nextstakes[0].balance" == 2498323.273015384
jsonpath "$.data.nextstakes[0].chainId" == "5f704c"
jsonpath "$.data.nextstakes[0].delegate" == "B62qiburnzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzmp7r7UN6X"
jsonpath "$.data.nextstakes[0].epoch" == 42
jsonpath "$.data.nextstakes[0].ledgerHash" == "jxYFH645cwMMMDmDe7KnvTuKJ5Ev8zZbWtA73fDFn7Jyh8p6SwH"
jsonpath "$.data.nextstakes[0].nonce" == 0
jsonpath "$.data.nextstakes[0].pk" == "B62qiburnzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzmp7r7UN6X"
jsonpath "$.data.nextstakes[0].public_key" == "B62qiburnzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzmp7r7UN6X"
jsonpath "$.data.nextstakes[0].token" == 1
jsonpath "$.data.nextstakes[0].voting_for" == "3NK2tkzqqK5spR2sZ7tujjqPksL45M3UUrcA4WhCkeiPtnugyE2x"
jsonpath "$.data.nextstakes[0].nextDelegationTotals.countDelegates" == 1
jsonpath "$.data.nextstakes[0].nextDelegationTotals.totalDelegated" == 2498323.273015384

# block production info
jsonpath "$.data.nextStakes[0].pk_epoch_num_blocks" == 0 # no blocks have been produced in the next epoch
jsonpath "$.data.nextStakes[0].pk_total_num_blocks" == 0 # no blocks have been produced in the next epoch
jsonpath "$.data.nextStakes[0].epoch_num_blocks" == 0    # no blocks have been produced in the next epoch
jsonpath "$.data.nextStakes[0].total_num_blocks" == 204

duration < 5000
