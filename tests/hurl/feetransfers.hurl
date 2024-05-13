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
jsonpath "$.data.feetransfers[0].stateHash" == "3NLNyQC4XgQX2Q9H7fC2UxFZKY4xwwUZop8jVR24SWYNNE93FsnS"
jsonpath "$.data.feetransfers[0].recipient" == "B62qjWwDxk5nGMXN32ffuMpMSp3wPa7BLB3AXHH5EYiczcRxDTF9uP4"
jsonpath "$.data.feetransfers[0].fee" == 10000000
jsonpath "$.data.feetransfers[0].type" == "Fee_transfer"
jsonpath "$.data.feetransfers[0].blockHeight" == 120
jsonpath "$.data.feetransfers[0].canonical" == true
jsonpath "$.data.feetransfers[0].dateTime" == "2021-03-17T08:39:00.000Z"
duration < 1000

POST {{url}}
```graphql
{
  feetransfers(sortBy: BLOCKHEIGHT_DESC, query: {recipient: "B62qjWwDxk5nGMXN32ffuMpMSp3wPa7BLB3AXHH5EYiczcRxDTF9uP4", canonical: true}) {
    fee
    recipient
    type
    dateTime
  }
}
```
HTTP 200
[Asserts]
jsonpath "$.data.feetransfers" count == 1
jsonpath "$.data.feetransfers[0].recipient" == "B62qjWwDxk5nGMXN32ffuMpMSp3wPa7BLB3AXHH5EYiczcRxDTF9uP4"
duration < 1000
