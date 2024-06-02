#
# Stakes ledger hash query
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
jsonpath "$.data.stakes[0].username" == null

# last datum
jsonpath "$.data.stakes[9].pk" == "B62qq3TQ8AP7MFYPVtMx5tZGF3kWLJukfwG1A1RGvaBW1jfTPTkDBW6"
jsonpath "$.data.stakes[9].balance" == 1104.686899999
jsonpath "$.data.stakes[9].balanceNanomina" == 1104686899999
jsonpath "$.data.stakes[9].username" == null


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

# TODO https://github.com/Granola-Team/mina-indexer/issues/934
# robinbb: Set to 1 min so that we get some idea of upper bound of wait time.
# 30 s was too little.
duration < 60000
