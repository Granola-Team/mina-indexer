#
# Fee transfers canonical block height descending query
#

POST {{url}}
```graphql
{
  feetransfers(
    limit: 101
    sortBy: BLOCKHEIGHT_DESC
    query: { canonical: true }
  ) {
    stateHash
    recipient
    fee
    type
    blockHeight
    canonical
    dateTime
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.feetransfers" count == 101

# first datum
jsonpath "$.data.feetransfers[0].stateHash" == "3NLNyQC4XgQX2Q9H7fC2UxFZKY4xwwUZop8jVR24SWYNNE93FsnS"
jsonpath "$.data.feetransfers[0].recipient" == "B62qjWwDxk5nGMXN32ffuMpMSp3wPa7BLB3AXHH5EYiczcRxDTF9uP4"
jsonpath "$.data.feetransfers[0].fee" == 10000000
jsonpath "$.data.feetransfers[0].type" == "Fee_transfer"
jsonpath "$.data.feetransfers[0].blockHeight" == 120
jsonpath "$.data.feetransfers[0].canonical" == true
jsonpath "$.data.feetransfers[0].dateTime" == "2021-03-17T08:39:00.000Z"

# last datum
jsonpath "$.data.feetransfers[100].stateHash" == "3NK6gZY6xoC98wTJdmfprmfWxeEU2o1mtFgKN9vh8z27LjnowLrz"
jsonpath "$.data.feetransfers[100].recipient" == "B62qkHM9NT3nDefqUvSMe8qnfEfeXipqkzZrvoBXpfaC9m2BdYjWVJA"
jsonpath "$.data.feetransfers[100].fee" == 720000000000
jsonpath "$.data.feetransfers[100].type" == "Coinbase"
jsonpath "$.data.feetransfers[100].blockHeight" == 70
jsonpath "$.data.feetransfers[100].canonical" == true
jsonpath "$.data.feetransfers[100].dateTime" == "2021-03-17T04:48:00.000Z"

duration < 2000

#
# Fee transfers canonical state hash block height descending query
#

POST {{url}}
```graphql
{
  feetransfers(
    sortBy: BLOCKHEIGHT_DESC
    query: {
      blockStateHash: {
        stateHash: "3NLNyQC4XgQX2Q9H7fC2UxFZKY4xwwUZop8jVR24SWYNNE93FsnS"
      }
      canonical: true
    }
  ) {
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
{
  feetransfers(
    limit: 100
    sortBy: BLOCKHEIGHT_DESC
    query: { canonical: true, blockHeight_lte: 120 }
  ) {
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
```
HTTP 200
[Asserts]

jsonpath "$.data.feetransfers" count == 100

jsonpath "$.data.feetransfers[0].blockHeight" == 120
jsonpath "$.data.feetransfers[99].blockHeight" == 70

duration < 2000

#
# Fee transfer epoch & total internal commands query
#

POST {{url}}
```graphql
{
  feetransfers(limit: 100, sortBy: BLOCKHEIGHT_ASC) {
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
jsonpath "$.data.feetransfers[0].epoch_num_internal_commands" == 196
jsonpath "$.data.feetransfers[0].total_num_internal_commands" == 399

duration < 500

#
# Fee transfer block state hash & block height LTE query
#

POST {{url}}
```graphql
{
  feetransfers(
    limit: 100
    sortBy: BLOCKHEIGHT_DESC
    query: {
      canonical: true
      blockHeight_lte: 100
      blockStateHash: {
        stateHash: "3NKrnCRmvomXqor8pnqrUsLv4XcofJBu8VWqAsWRirGNPszo1a66"
      }
    }
  ) {
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
```
HTTP 200
[Asserts]

jsonpath "$.data.feetransfers" count == 0

#
# Fee transfers for a given recipient and block height
#

POST {{url}}
```graphql
{
  feetransfers(
    limit: 25
    sortBy: BLOCKHEIGHT_DESC
    query: {
      recipient: "B62qkHM9NT3nDefqUvSMe8qnfEfeXipqkzZrvoBXpfaC9m2BdYjWVJA"
      canonical: true
      blockHeight_lte: 100
    }
  ) {
    recipient
    blockHeight
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
{
  feetransfers(
    limit: 4
    sortBy: BLOCKHEIGHT_DESC
    query: {
      recipient: "B62qkiJuTwdJBARAPGAvStuEa37kZVZPyDrQoUCuM7WQUmZZydNBmTf"
      canonical: true
      blockHeight_lte: 120
    }
  ) {
    recipient
    blockHeight
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

duration < 500
