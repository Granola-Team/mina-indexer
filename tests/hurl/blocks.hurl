POST {{url}}
```graphql
query Blocks($limit: Int = 10, $sort_by: BlockSortByInput!, $query: BlockQueryInput!) {
  blocks(limit: $limit, sortBy: $sort_by, query: $query ) {
    stateHash
    txFees
    blockHeight
    canonical
    creator
    dateTime
    receivedTime
    snarkFees
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
jsonpath "$.data.blocks[0].stateHash" == "3NKLtRnMaWAAfRvdizaeaucDPBePPKGbKw64RVcuRFtMMkE8aAD4"
jsonpath "$.data.blocks[0].txFees" == "10000000"
jsonpath "$.data.blocks[0].blockHeight" == 100
jsonpath "$.data.blocks[0].canonical" == true
jsonpath "$.data.blocks[0].creator" == "B62qrxNgwAdhGYZv1BXQRt2HgopUceFyrtXZMikwsuaHu5FigRJjhwY"
jsonpath "$.data.blocks[0].dateTime" == "2021-03-17T07:15:00.000Z"
jsonpath "$.data.blocks[0].receivedTime" == "2021-03-17T07:15:32.734Z"
jsonpath "$.data.blocks[0].snarkFees" == "0"

duration < 1000

