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
  "limit": 120,
  "sort_by": "BLOCKHEIGHT_ASC",
  "query": {
    "canonical": true
  }
}
```
HTTP 200
[Asserts]
jsonpath "$.data.blocks[0].stateHash" == "3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ"
jsonpath "$.data.blocks[0].blockHeight" == 1
jsonpath "$.data.blocks[0].txFees" == "0"
jsonpath "$.data.blocks[0].canonical" == true
jsonpath "$.data.blocks[0].creator" == "B62qiy32p8kAKnny8ZFwoMhYpBppM1DWVCqAPBYNcXnsAHhnfAAuXgg"
jsonpath "$.data.blocks[0].dateTime" == "2021-03-17T00:00:00.000Z"
jsonpath "$.data.blocks[0].receivedTime" == "2021-03-17T00:00:00.000Z"
jsonpath "$.data.blocks[0].snarkFees" == "0"

jsonpath "$.data.blocks[119].stateHash" == "3NLNyQC4XgQX2Q9H7fC2UxFZKY4xwwUZop8jVR24SWYNNE93FsnS"
jsonpath "$.data.blocks[119].txFees" == "10000000"
jsonpath "$.data.blocks[119].blockHeight" == 120
jsonpath "$.data.blocks[119].canonical" == true
jsonpath "$.data.blocks[119].creator" == "B62qjWwDxk5nGMXN32ffuMpMSp3wPa7BLB3AXHH5EYiczcRxDTF9uP4"
jsonpath "$.data.blocks[119].dateTime" == "2021-03-17T08:39:00.000Z"
jsonpath "$.data.blocks[119].receivedTime" == "2021-03-17T08:39:40.591Z"
jsonpath "$.data.blocks[119].snarkFees" == "0"

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

POST {{url}}
```graphql
{
  blocks(query: {stateHash: "3NKd5So3VNqGZtRZiWsti4yaEe1fX79yz5TbfG6jBZqgMnCQQp3R"}) {
    stateHash
    transactions {
      coinbase
      coinbaseReceiverAccount {
        publicKey
      }
      feeTransfer {
        type
        recipient
        fee
      }
      userCommands {
        failureReason
        hash
        kind
        from
        to
        amount
        fee
        blockHeight
      }
    }
  }
}
```
HTTP 200
[Asserts]
jsonpath "$.data.blocks[0].stateHash" == "3NKd5So3VNqGZtRZiWsti4yaEe1fX79yz5TbfG6jBZqgMnCQQp3R"
jsonpath "$.data.blocks[0].transactions.coinbase" == 720000000000
jsonpath "$.data.blocks[0].transactions.coinbaseReceiverAccount.publicKey" == "B62qqSUUCnoC8Vehw5xwhrnaNxhk6Xe3FcBhngoxyXCbJBfvVhiqia1"
jsonpath "$.data.blocks[0].transactions.feeTransfer" count == 2
jsonpath "$.data.blocks[0].transactions.feeTransfer[0].type" == "Coinbase"
jsonpath "$.data.blocks[0].transactions.feeTransfer[0].recipient" == "B62qqSUUCnoC8Vehw5xwhrnaNxhk6Xe3FcBhngoxyXCbJBfvVhiqia1"
jsonpath "$.data.blocks[0].transactions.feeTransfer[0].fee" == "720000000000"
jsonpath "$.data.blocks[0].transactions.userCommands" count == 4
jsonpath "$.data.blocks[0].transactions.userCommands[0].kind" == "PAYMENT"
jsonpath "$.data.blocks[0].transactions.userCommands[0].from" == "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy"
jsonpath "$.data.blocks[0].transactions.userCommands[0].to" == "B62qjYanmV7y9njVeH5UHkz3GYBm7xKir1rAnoY4KsEYUGLMiU45FSM"
jsonpath "$.data.blocks[0].transactions.userCommands[0].amount" == 0.000001
jsonpath "$.data.blocks[0].transactions.userCommands[0].fee" == 0.01
jsonpath "$.data.blocks[0].transactions.userCommands[0].blockHeight" == 3

duration < 1000