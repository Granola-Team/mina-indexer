#
# Internal commands canonical block height descending query
#

POST {{url}}
```graphql
{
  internalCommands(
    limit: 101
    sortBy: BLOCK_HEIGHT_DESC
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
jsonpath "$.data.internalCommands" count == 101

# first datum
jsonpath "$.data.internalCommands[0].stateHash" == "3NLNyQC4XgQX2Q9H7fC2UxFZKY4xwwUZop8jVR24SWYNNE93FsnS"
jsonpath "$.data.internalCommands[0].recipient" == "B62qjWwDxk5nGMXN32ffuMpMSp3wPa7BLB3AXHH5EYiczcRxDTF9uP4"
jsonpath "$.data.internalCommands[0].fee" == 10000000
jsonpath "$.data.internalCommands[0].type" == "Fee_transfer"
jsonpath "$.data.internalCommands[0].blockHeight" == 120
jsonpath "$.data.internalCommands[0].canonical" == true
jsonpath "$.data.internalCommands[0].dateTime" == "2021-03-17T08:39:00.000Z"

# last datum
jsonpath "$.data.internalCommands[100].stateHash" == "3NK6gZY6xoC98wTJdmfprmfWxeEU2o1mtFgKN9vh8z27LjnowLrz"
jsonpath "$.data.internalCommands[100].recipient" == "B62qkHM9NT3nDefqUvSMe8qnfEfeXipqkzZrvoBXpfaC9m2BdYjWVJA"
jsonpath "$.data.internalCommands[100].fee" == 720000000000
jsonpath "$.data.internalCommands[100].type" == "Coinbase"
jsonpath "$.data.internalCommands[100].blockHeight" == 70
jsonpath "$.data.internalCommands[100].canonical" == true
jsonpath "$.data.internalCommands[100].dateTime" == "2021-03-17T04:48:00.000Z"

duration < 2000

#
# Internal commands canonical state hash block height descending query
#

POST {{url}}
```graphql
{
  internalCommands(
    sortBy: BLOCK_HEIGHT_DESC
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
jsonpath "$.data.internalCommands" count == 2

# only datum
jsonpath "$.data.internalCommands[0].recipient" == "B62qjWwDxk5nGMXN32ffuMpMSp3wPa7BLB3AXHH5EYiczcRxDTF9uP4"
jsonpath "$.data.internalCommands[0].blockStateHash.stateHash" == "3NLNyQC4XgQX2Q9H7fC2UxFZKY4xwwUZop8jVR24SWYNNE93FsnS"
jsonpath "$.data.internalCommands[0].blockStateHash.total_num_blocks" == 204
jsonpath "$.data.internalCommands[0].type" == "Coinbase"

duration < 1000

#
# Internal command block height LTE query
#

POST {{url}}
```graphql
{
  internalCommands(
    limit: 100
    sortBy: BLOCK_HEIGHT_DESC
    query: { canonical: true, blockHeightLte: 120 }
  ) {
    blockHeight
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.internalCommands" count == 100

# first datum
jsonpath "$.data.internalCommands[0].blockHeight" == 120

# last datum
jsonpath "$.data.internalCommands[99].blockHeight" == 70

duration < 2000

#
# Internal command epoch & total internal commands query
#

POST {{url}}
```graphql
{
  internalCommands(limit: 100, sortBy: BLOCK_HEIGHT_ASC) {
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
jsonpath "$.data.internalCommands" count == 100

# first datum
jsonpath "$.data.internalCommands[0].blockHeight" == 2
jsonpath "$.data.internalCommands[0].blockStateHash.stateHash" == "3NLyWnjZqUECniE1q719CoLmes6WDQAod4vrTeLfN7XXJbHv6EHH"
jsonpath "$.data.internalCommands[0].epoch_num_internal_commands" == 399
jsonpath "$.data.internalCommands[0].total_num_internal_commands" == 399

# last datum
jsonpath "$.data.internalCommands[99].blockHeight" == 31
jsonpath "$.data.internalCommands[99].blockStateHash.stateHash" == "3NKQzfWDJSQkYrNC61nrMqMgJiVvrsvyF2e8Xtx18Vt4LvbasE9Z"
jsonpath "$.data.internalCommands[99].epoch_num_internal_commands" == 399
jsonpath "$.data.internalCommands[99].total_num_internal_commands" == 399

duration < 1000

#
# Internal command block state hash & block height LTE query
#

POST {{url}}
```graphql
{
  internalCommands(
    limit: 100
    sortBy: BLOCK_HEIGHT_DESC
    query: {
      canonical: true
      blockHeightLte: 100
      blockStateHash: {
        stateHash: "3NKrnCRmvomXqor8pnqrUsLv4XcofJBu8VWqAsWRirGNPszo1a66"
      }
    }
  ) {
    blockHeight
  }
}
```
HTTP 200
[Asserts]

jsonpath "$.data.internalCommands" count == 0

#
# Internal commands for a given recipient and block height
#

POST {{url}}
```graphql
{
  internalCommands(
    limit: 25
    sortBy: BLOCK_HEIGHT_DESC
    query: {
      recipient: "B62qkHM9NT3nDefqUvSMe8qnfEfeXipqkzZrvoBXpfaC9m2BdYjWVJA"
      canonical: true
      blockHeightLte: 100
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
jsonpath "$.data.internalCommands" count == 2

# first datum
jsonpath "$.data.internalCommands[0].recipient" == "B62qkHM9NT3nDefqUvSMe8qnfEfeXipqkzZrvoBXpfaC9m2BdYjWVJA"
jsonpath "$.data.internalCommands[0].blockHeight" == 70

# last datum
jsonpath "$.data.internalCommands[1].recipient" == "B62qkHM9NT3nDefqUvSMe8qnfEfeXipqkzZrvoBXpfaC9m2BdYjWVJA"
jsonpath "$.data.internalCommands[1].blockHeight" == 70

duration < 1000

#
# Internal commands for a given recipient and block height, respecting the limit
#

POST {{url}}
```graphql
{
  internalCommands(
    limit: 4
    sortBy: BLOCK_HEIGHT_DESC
    query: {
      recipient: "B62qkiJuTwdJBARAPGAvStuEa37kZVZPyDrQoUCuM7WQUmZZydNBmTf"
      canonical: true
      blockHeightLte: 120
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
jsonpath "$.data.internalCommands" count == 4

# all data
jsonpath "$.data.internalCommands[0].recipient" == "B62qkiJuTwdJBARAPGAvStuEa37kZVZPyDrQoUCuM7WQUmZZydNBmTf"
jsonpath "$.data.internalCommands[0].blockHeight" == 109

jsonpath "$.data.internalCommands[1].recipient" == "B62qkiJuTwdJBARAPGAvStuEa37kZVZPyDrQoUCuM7WQUmZZydNBmTf"
jsonpath "$.data.internalCommands[1].blockHeight" == 109

jsonpath "$.data.internalCommands[2].recipient" == "B62qkiJuTwdJBARAPGAvStuEa37kZVZPyDrQoUCuM7WQUmZZydNBmTf"
jsonpath "$.data.internalCommands[2].blockHeight" == 97

jsonpath "$.data.internalCommands[3].recipient" == "B62qkiJuTwdJBARAPGAvStuEa37kZVZPyDrQoUCuM7WQUmZZydNBmTf"
jsonpath "$.data.internalCommands[3].blockHeight" == 97

duration < 500
