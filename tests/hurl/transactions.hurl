POST {{url}}
```graphql
query Transactions($limit: Int, $sort_by: TransactionSortByInput!, $query: TransactionQueryInput!) {
  transactions(limit: $limit, sortBy: $sort_by, query: $query ) {
    blockHeight
    canonical
    amount
    fee
    kind
    to
    from
    nonce
    memo
    hash
    block {
      dateTime
      stateHash
    }
    receiver {
      publicKey
    }
  }
}
variables {
  "limit": 50,
  "sort_by": "BLOCKHEIGHT_ASC",
  "query": {
    "canonical": true
  }
}
```
HTTP 200
[Asserts]
jsonpath "$.data.transactions[0].blockHeight" == 3
jsonpath "$.data.transactions[0].memo" == ""
jsonpath "$.data.transactions[0].hash" == "CkpYeG32dVJUjs6iq3oroXWitXar1eBtV3GVFyH5agw7HPp9bG4yQ"

jsonpath "$.data.transactions[49].blockHeight" == 59
jsonpath "$.data.transactions[49].memo" == ""
jsonpath "$.data.transactions[49].hash" == "CkpZ3EzeQLT4m3mLUwyKFdvF3cjvvFGUzR9jug14E8bFryTpTsxYz"

duration < 1000
