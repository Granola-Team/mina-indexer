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
jsonpath "$.data.blocks[0].stateHash" == "3NLNyQC4XgQX2Q9H7fC2UxFZKY4xwwUZop8jVR24SWYNNE93FsnS"
jsonpath "$.data.blocks[0].txFees" == "10000000"
jsonpath "$.data.blocks[0].blockHeight" == 120
jsonpath "$.data.blocks[0].canonical" == true
jsonpath "$.data.blocks[0].creator" == "B62qjWwDxk5nGMXN32ffuMpMSp3wPa7BLB3AXHH5EYiczcRxDTF9uP4"
jsonpath "$.data.blocks[0].dateTime" == "2021-03-17T08:39:00.000Z"
jsonpath "$.data.blocks[0].receivedTime" == "2021-03-17T08:39:40.591Z"
jsonpath "$.data.blocks[0].snarkFees" == "0"

duration < 1000

POST {{url}}
```graphql
{
  blocks(query: {creatorAccount: {publicKey: "B62qnYBehkVZzgJBCC5yhFyF2L7mMRPsVnKW2xpcWFVuQYRVgoqwDCS"}}) {
    stateHash
    blockHeight
    winnerAccount {
      publicKey
    }
  }
}
```
HTTP 200
[Asserts]
jsonpath "$.data.blocks" count == 3
