#
# Fee transfers canonical block height descending query
#

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
  "limit": 100,
  "sort_by": "BLOCKHEIGHT_DESC",
  "query": {
    "canonical": true
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.feetransfers" count == 100

# first datum
jsonpath "$.data.feetransfers[0].stateHash" == "3NLNyQC4XgQX2Q9H7fC2UxFZKY4xwwUZop8jVR24SWYNNE93FsnS"
jsonpath "$.data.feetransfers[0].recipient" == "B62qjWwDxk5nGMXN32ffuMpMSp3wPa7BLB3AXHH5EYiczcRxDTF9uP4"
jsonpath "$.data.feetransfers[0].fee" == 10000000
jsonpath "$.data.feetransfers[0].type" == "Fee_transfer"
jsonpath "$.data.feetransfers[0].blockHeight" == 120
jsonpath "$.data.feetransfers[0].canonical" == true
jsonpath "$.data.feetransfers[0].dateTime" == "2021-03-17T08:39:00.000Z"

# last datum
jsonpath "$.data.feetransfers[99].stateHash" == "3NKKJqqZovA2TnivikmkZjMtgcewSPBkXsJz5UqvwrFttMhRsztJ"
jsonpath "$.data.feetransfers[99].recipient" == "B62qrdhG66vK71Jbdz6Xs7cnDxQ8f6jZUFvefkp3pje4EejYUTvotGP"
jsonpath "$.data.feetransfers[99].fee" == 1440000000000
jsonpath "$.data.feetransfers[99].type" == "Coinbase"
jsonpath "$.data.feetransfers[99].blockHeight" == 71
jsonpath "$.data.feetransfers[99].canonical" == true
jsonpath "$.data.feetransfers[99].dateTime" == "2021-03-17T04:51:00.000Z"

duration < 2000

#
# Fee transfers canonical state hash block height descending query
#

POST {{url}}
```graphql
{
  feetransfers(sortBy: BLOCKHEIGHT_DESC, query: {blockStateHash: {stateHash: "3NLNyQC4XgQX2Q9H7fC2UxFZKY4xwwUZop8jVR24SWYNNE93FsnS"}, canonical: true}) {
    recipient
    type
    blockStateHash {
      stateHash
      total_num_blocks
    }
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.feetransfers" count == 2

# only datum
jsonpath "$.data.feetransfers[0].recipient" == "B62qjWwDxk5nGMXN32ffuMpMSp3wPa7BLB3AXHH5EYiczcRxDTF9uP4"
jsonpath "$.data.feetransfers[0].blockStateHash.stateHash" == "3NLNyQC4XgQX2Q9H7fC2UxFZKY4xwwUZop8jVR24SWYNNE93FsnS"
jsonpath "$.data.feetransfers[0].blockStateHash.total_num_blocks" == 204
jsonpath "$.data.feetransfers[0].type" == "Coinbase"

duration < 1000

#
# Fee transfer block height LTE query
#

POST {{url}}
```graphql
query InternalCommandsQuery(
  $sort_by: FeetransferSortByInput!
  $limit: Int = 10
  $query: FeetransferQueryInput!
) {
  feetransfers(sortBy: $sort_by, limit: $limit, query: $query) {
    blockHeight
    blockStateHash {
      stateHash
    }
    fee
    recipient
    type
    dateTime
    canonical
  }
}

variables {
  "sort_by": "BLOCKHEIGHT_DESC",
  "limit": 100,
  "query": {
    "canonical": true,
    "blockHeight_lte": 120
  }
}
```
HTTP 200
[Asserts]

jsonpath "$.data.feetransfers" count == 100

jsonpath "$.data.feetransfers[0].blockHeight" == 120
jsonpath "$.data.feetransfers[99].blockHeight" == 71

duration < 2000

#
# Fee transfer epoch & total internal commands query
#

POST {{url}}
```graphql
{
  feetransfers(sortBy: "BLOCKHEIGHT_ASC", limit: 100) {
    blockHeight
    blockStateHash {
      stateHash
    }
    epoch_num_internal_commands
    total_num_internal_commands
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.feetransfers" count == 100

# first datum
jsonpath "$.data.feetransfers[0].blockHeight" == 2
jsonpath "$.data.feetransfers[0].blockStateHash.stateHash" == "3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH"
jsonpath "$.data.feetransfers[0].epoch_num_internal_commands" == 205
jsonpath "$.data.feetransfers[0].total_num_internal_commands" == 205

duration < 1000

#
# Fee transfer block state hash & block height LTE query
#

POST {{url}}
```graphql
query InternalCommandsQuery(
  $sort_by: FeetransferSortByInput!
  $limit: Int = 10
  $query: FeetransferQueryInput!
) {
  feetransfers(sortBy: $sort_by, limit: $limit, query: $query) {
    blockHeight
    blockStateHash {
      stateHash
    }
    fee
    recipient
    type
    dateTime
    canonical
  }
}

variables {
  "sort_by": "BLOCKHEIGHT_DESC",
  "limit": 100,
  "query": {
    "canonical": true,
    "blockHeight_lte": 100,
    "blockStateHash": {
      "stateHash": "3NKrnCRmvomXqor8pnqrUsLv4XcofJBu8VWqAsWRirGNPszo1a66"
    }
  }
}
```
HTTP 200
[Asserts]

jsonpath "$.data.feetransfers" count == 0

#
# Fee transfers for a given recipient and block height
#

POST {{url}}
```graphql
query Feetransfers($limit: Int = 10, $sort_by: FeetransferSortByInput!, $query: FeetransferQueryInput!) {
  feetransfers(limit: $limit, sortBy: $sort_by, query: $query ) {
    recipient
    blockHeight
  }
}

variables {
  "sort_by": "BLOCKHEIGHT_DESC",
  "limit": 25,
  "query": {
    "recipient": "B62qkHM9NT3nDefqUvSMe8qnfEfeXipqkzZrvoBXpfaC9m2BdYjWVJA",
    "canonical": true,
    "blockHeight_lte": 100
  }
}

```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.feetransfers" count == 2

jsonpath "$.data.feetransfers[0].recipient" == "B62qkHM9NT3nDefqUvSMe8qnfEfeXipqkzZrvoBXpfaC9m2BdYjWVJA"
jsonpath "$.data.feetransfers[0].blockHeight" == 70
jsonpath "$.data.feetransfers[1].recipient" == "B62qkHM9NT3nDefqUvSMe8qnfEfeXipqkzZrvoBXpfaC9m2BdYjWVJA"
jsonpath "$.data.feetransfers[1].blockHeight" == 70

duration < 1000

#
# Fee transfers for a given recipient and block height, respecting the limit
#

POST {{url}}
```graphql
query Feetransfers($limit: Int = 10, $sort_by: FeetransferSortByInput!, $query: FeetransferQueryInput!) {
  feetransfers(limit: $limit, sortBy: $sort_by, query: $query ) {
    recipient
    blockHeight
  }
}

variables {
  "sort_by": "BLOCKHEIGHT_DESC",
  "limit": 4,
  "query": {
    "recipient": "B62qkiJuTwdJBARAPGAvStuEa37kZVZPyDrQoUCuM7WQUmZZydNBmTf",
    "canonical": true,
    "blockHeight_lte": 120
  }
}

```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.feetransfers" count == 4

jsonpath "$.data.feetransfers[0].recipient" == "B62qkiJuTwdJBARAPGAvStuEa37kZVZPyDrQoUCuM7WQUmZZydNBmTf"
jsonpath "$.data.feetransfers[0].blockHeight" == 109
jsonpath "$.data.feetransfers[1].recipient" == "B62qkiJuTwdJBARAPGAvStuEa37kZVZPyDrQoUCuM7WQUmZZydNBmTf"
jsonpath "$.data.feetransfers[1].blockHeight" == 109
jsonpath "$.data.feetransfers[2].recipient" == "B62qkiJuTwdJBARAPGAvStuEa37kZVZPyDrQoUCuM7WQUmZZydNBmTf"
jsonpath "$.data.feetransfers[2].blockHeight" == 97
jsonpath "$.data.feetransfers[3].recipient" == "B62qkiJuTwdJBARAPGAvStuEa37kZVZPyDrQoUCuM7WQUmZZydNBmTf"
jsonpath "$.data.feetransfers[3].blockHeight" == 97

duration < 1000
