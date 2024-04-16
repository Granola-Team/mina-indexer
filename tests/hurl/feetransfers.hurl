POST {{url}}
```graphql
query Feetransfers($limit: Int = 10, $sort_by: FeetransferSortByInput!, $query: FeetransferQueryInput!) {
  feetransfers(limit: $limit, sortBy: $sort_by, query: $query ) {
    stateHash
    recipient
    fee
    type
    blockHeight
    canonical
    dateTime
  }
}
variables {
  "limit": 50,
  "sort_by": "BLOCKHEIGHT_DESC",
  "query": {
    "canonical": true
  }
}
```
HTTP 200
[Asserts]
jsonpath "$.data.feetransfers[0].stateHash" == "3NKLtRnMaWAAfRvdizaeaucDPBePPKGbKw64RVcuRFtMMkE8aAD4"
jsonpath "$.data.feetransfers[0].recipient" == "B62qqa9g4CFfkSuX2j22S52z6UfcDcS9tMTgQrFKZ21v7GrEP6Zu5Tc"
jsonpath "$.data.feetransfers[0].fee" == 10000000
jsonpath "$.data.feetransfers[0].type" == "Fee_transfer"
jsonpath "$.data.feetransfers[0].blockHeight" == 100
jsonpath "$.data.feetransfers[0].canonical" == true
jsonpath "$.data.feetransfers[0].dateTime" == "2021-03-17T07:15:00.000Z"
duration < 1000
