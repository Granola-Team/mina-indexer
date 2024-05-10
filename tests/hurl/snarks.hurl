POST {{url}}
```graphql
query Snarks($limit: Int = 10, $sort_by: SnarkSortByInput!, $query: SnarkQueryInput!) {
  snarks(limit: $limit, sortBy: $sort_by, query: $query ) {
    canonical
    blockHeight
    prover
    fee
    dateTime
    block {
      stateHash
    }
  }
}
variables {
  "limit": 10,
  "sort_by": "BLOCKHEIGHT_DESC",
  "query": {
    "canonical": true
  }
}
```
HTTP 200
[Asserts]
jsonpath "$.data.snarks[0].dateTime" == "2021-03-17T08:06:00.000Z"
jsonpath "$.data.snarks[0].canonical" == true
jsonpath "$.data.snarks[0].blockHeight" == 111
jsonpath "$.data.snarks[0].prover" == "B62qrCz3ehCqi8Pn8y3vWC9zYEB9RKsidauv15DeZxhzkxL3bKeba5h"
jsonpath "$.data.snarks[0].fee" == 0
jsonpath "$.data.snarks[0].block.stateHash" == "3NL33j16AWm3Jhjj1Ud25E54hu7HpUq4WBQcAiijEKMfXqwFJwzK"

duration < 1000
