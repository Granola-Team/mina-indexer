# 
# Blocks canonical height ascending query
# 

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

# total query results
jsonpath "$.data.blocks" count == 120

# first datum
jsonpath "$.data.blocks[0].stateHash" == "3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ"
jsonpath "$.data.blocks[0].blockHeight" == 1
jsonpath "$.data.blocks[0].txFees" == "0"
jsonpath "$.data.blocks[0].canonical" == true
jsonpath "$.data.blocks[0].creator" == "B62qiy32p8kAKnny8ZFwoMhYpBppM1DWVCqAPBYNcXnsAHhnfAAuXgg"
jsonpath "$.data.blocks[0].dateTime" == "2021-03-17T00:00:00.000Z"
jsonpath "$.data.blocks[0].receivedTime" == "2021-03-17T00:00:00.000Z"
jsonpath "$.data.blocks[0].snarkFees" == "0"

# last datum
jsonpath "$.data.blocks[119].stateHash" == "3NLNyQC4XgQX2Q9H7fC2UxFZKY4xwwUZop8jVR24SWYNNE93FsnS"
jsonpath "$.data.blocks[119].txFees" == "10000000"
jsonpath "$.data.blocks[119].blockHeight" == 120
jsonpath "$.data.blocks[119].canonical" == true
jsonpath "$.data.blocks[119].creator" == "B62qjWwDxk5nGMXN32ffuMpMSp3wPa7BLB3AXHH5EYiczcRxDTF9uP4"
jsonpath "$.data.blocks[119].dateTime" == "2021-03-17T08:39:00.000Z"
jsonpath "$.data.blocks[119].receivedTime" == "2021-03-17T08:39:40.591Z"
jsonpath "$.data.blocks[119].snarkFees" == "0"

duration < 1000

# 
# Blocks non-canonical height descending query
# 

POST {{url}}
```graphql
{
  blocks(limit: 100, sortBy: "BLOCKHEIGHT_DESC", query: {canonical: false}) {
    stateHash
    blockHeight
    canonical
  } 
}
```
HTTP 200
[Asserts]

# Non-canonical block count
jsonpath "$.data.blocks" count == 84

# First non-canonical block
jsonpath "$.data.blocks[0].blockHeight" == 120
jsonpath "$.data.blocks[0].stateHash" == "3NKFoxsSXhZ5YVfAUWgkEh6FXdATjHgUxfjTnGtJcrTuUZavq7d1"
jsonpath "$.data.blocks[0].canonical" == false

# Last non-canonical block
jsonpath "$.data.blocks[83].blockHeight" == 6
jsonpath "$.data.blocks[83].stateHash" == "3NKqMEewA8gvEiW7So7nZ3DN6tPnmCtHpWuAzADN5ff9wiqkGf45"
jsonpath "$.data.blocks[83].canonical" == false

duration < 1000

# 
# Blocks creator account  query (default sort_by: BLOCKHEIGHT_DESC)
# 

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

# Total blocks produced
jsonpath "$.data.blocks" count == 3

# Block data
jsonpath "$.data.blocks[0].stateHash" == "3NKGUXQYm1nF1hdEDEzwuLRWZjBTdSeKqHfqS3JPCzJ2i31mrovV"
jsonpath "$.data.blocks[0].blockHeight" == 109

jsonpath "$.data.blocks[1].stateHash" == "3NLgPYVDSV6y7ZKSQrjowedjZn9RuLwdx311vheWZEy3mBjKDbmj"
jsonpath "$.data.blocks[1].blockHeight" == 97

jsonpath "$.data.blocks[2].stateHash" == "3NKQUoBfi9vkbuqtDJmSEYBQrcSo4GjwG8bPCiii4yqM8AxEQvtY"
jsonpath "$.data.blocks[2].blockHeight" == 5

duration < 2000

# 
# Blocks coinbase receiver height ascending query
# 

POST {{url}}
```graphql
{
  blocks(query: {coinbaseReceiver: {public_key: "B62qnXMPtWdK4hRyaemZrvNoDGe3pMQNeubQemMZVXetuEY6MfgYszF"}}, sortBy: "BLOCKHEIGHT_ASC") {
    stateHash
    blockHeight
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.blocks" count == 5

# data values
jsonpath "$.data.blocks[0].stateHash" == "3NLVZQz4FwFbvW4hejfyRpw5NyP8XvQjhj4wSsCjCKdHNBjwWsPG"
jsonpath "$.data.blocks[0].blockHeight" == 8

jsonpath "$.data.blocks[1].stateHash" == "3NKBupxvxcso3xiWQr3f4fzPJQhSh9efLJMoA49aqKbrjC3saRsv"
jsonpath "$.data.blocks[1].blockHeight" == 62

jsonpath "$.data.blocks[2].stateHash" == "3NLLW8wvytn913FtfqvqETPfJWW9GEGqEYTqYKSriAKGXUFqqnDi"
jsonpath "$.data.blocks[2].blockHeight" == 70

jsonpath "$.data.blocks[3].stateHash" == "3NKF3gkPkEAa6Nozz3bB4fekkrMi1nXmECioJ9SjrcbF5BKK7jDh"
jsonpath "$.data.blocks[3].blockHeight" == 72

jsonpath "$.data.blocks[4].stateHash" == "3NLftXM1dfzGJwLef5NCk974CXq8oXtAaQAkkhpJZxpaBwLFiemo"
jsonpath "$.data.blocks[4].blockHeight" == 84


duration < 2000

# 
# Blocks specific height query (ordered canonical first)
# 

POST {{url}}
```graphql
{
  blocks(query: {blockHeight: 6}) {
    stateHash
    canonical
    blockHeight
  }
}
```
HTTP 200
[Asserts]

# total blocks height 6
jsonpath "$.data.blocks" count == 4

# first block height 6
jsonpath "$.data.blocks[0].stateHash" == "3NKqRR2BZFV7Ad5kxtGKNNL59neXohf4ZEC5EMKrrnijB1jy4R5v"
jsonpath "$.data.blocks[0].blockHeight" == 6
jsonpath "$.data.blocks[0].canonical" == true

jsonpath "$.data.blocks[1].stateHash" == "3NKvdydTvLVDJ9PKAXrisjsXoZQvUy1V2sbComWyB2uyhARCJZ5M"
jsonpath "$.data.blocks[1].blockHeight" == 6
jsonpath "$.data.blocks[1].canonical" == false

jsonpath "$.data.blocks[2].stateHash" == "3NLM3k3Vk1qs36hZWdbWvi4sqwer3skbgPyHMWrZMBoscNLyjnY2"
jsonpath "$.data.blocks[2].blockHeight" == 6
jsonpath "$.data.blocks[2].canonical" == false

jsonpath "$.data.blocks[3].stateHash" == "3NKqMEewA8gvEiW7So7nZ3DN6tPnmCtHpWuAzADN5ff9wiqkGf45"
jsonpath "$.data.blocks[3].blockHeight" == 6
jsonpath "$.data.blocks[3].canonical" == false

duration < 2000

# 
# Blocks global slot query (ordered canonical first)
# 

POST {{url}}
```graphql
{
  blocks(query: {protocolState: {consensusState: {slotSinceGenesis: 169}}}) {
    stateHash
    canonical
    blockHeight
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.blocks" count == 3

# data values
jsonpath "$.data.blocks[0].stateHash" == "3NKpj5NkT4zrqAy1XPjWLe6BJuyprKEbu89kbxXxFYexabtEGKu1"
jsonpath "$.data.blocks[0].blockHeight" == 117
jsonpath "$.data.blocks[0].canonical" == true

jsonpath "$.data.blocks[1].stateHash" == "3NLgbVt67xbbKxJYrvhAmQbQeKcdf8fhwkJZjoAWsB6mS67V5gmB"
jsonpath "$.data.blocks[1].blockHeight" == 117
jsonpath "$.data.blocks[1].canonical" == false

jsonpath "$.data.blocks[2].stateHash" == "3NLCUCD6Vbg1yvaSe4YmGPY3nwSRkdwiQdVexvSLe7xgeVZA21Pp"
jsonpath "$.data.blocks[2].blockHeight" == 117
jsonpath "$.data.blocks[2].canonical" == false

duration < 2000

# 
# Blocks state hash transactions, fee transfers, user commands query
# 

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

# Unique block at state hash
jsonpath "$.data.blocks" count == 1

# state hash
jsonpath "$.data.blocks[0].stateHash" == "3NKd5So3VNqGZtRZiWsti4yaEe1fX79yz5TbfG6jBZqgMnCQQp3R"

# txns
# - coinbase
jsonpath "$.data.blocks[0].transactions.coinbase" == "720000000000"
jsonpath "$.data.blocks[0].transactions.coinbaseReceiverAccount.publicKey" == "B62qqSUUCnoC8Vehw5xwhrnaNxhk6Xe3FcBhngoxyXCbJBfvVhiqia1"

# - fee transfer(s)
jsonpath "$.data.blocks[0].transactions.feeTransfer" count == 1
jsonpath "$.data.blocks[0].transactions.feeTransfer[0].type" == "Fee_transfer"
jsonpath "$.data.blocks[0].transactions.feeTransfer[0].recipient" == "B62qqSUUCnoC8Vehw5xwhrnaNxhk6Xe3FcBhngoxyXCbJBfvVhiqia1"
jsonpath "$.data.blocks[0].transactions.feeTransfer[0].fee" == "120000000"

# - user commands
jsonpath "$.data.blocks[0].transactions.userCommands" count == 4

jsonpath "$.data.blocks[0].transactions.userCommands[0].kind" == "PAYMENT"
jsonpath "$.data.blocks[0].transactions.userCommands[0].from" == "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy"
jsonpath "$.data.blocks[0].transactions.userCommands[0].to" == "B62qjYanmV7y9njVeH5UHkz3GYBm7xKir1rAnoY4KsEYUGLMiU45FSM"
jsonpath "$.data.blocks[0].transactions.userCommands[0].amount" == 1000
jsonpath "$.data.blocks[0].transactions.userCommands[0].fee" == 10000000
jsonpath "$.data.blocks[0].transactions.userCommands[0].blockHeight" == 3

jsonpath "$.data.blocks[0].transactions.userCommands[1].kind" == "PAYMENT"
jsonpath "$.data.blocks[0].transactions.userCommands[1].from" == "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy"
jsonpath "$.data.blocks[0].transactions.userCommands[1].to" == "B62qjYanmV7y9njVeH5UHkz3GYBm7xKir1rAnoY4KsEYUGLMiU45FSM"
jsonpath "$.data.blocks[0].transactions.userCommands[1].amount" == 10000
jsonpath "$.data.blocks[0].transactions.userCommands[1].fee" == 50000000
jsonpath "$.data.blocks[0].transactions.userCommands[1].blockHeight" == 3

jsonpath "$.data.blocks[0].transactions.userCommands[2].kind" == "PAYMENT"
jsonpath "$.data.blocks[0].transactions.userCommands[2].from" == "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy"
jsonpath "$.data.blocks[0].transactions.userCommands[2].to" == "B62qjYanmV7y9njVeH5UHkz3GYBm7xKir1rAnoY4KsEYUGLMiU45FSM"
jsonpath "$.data.blocks[0].transactions.userCommands[2].amount" == 1000
jsonpath "$.data.blocks[0].transactions.userCommands[2].fee" == 10000000
jsonpath "$.data.blocks[0].transactions.userCommands[2].blockHeight" == 3

jsonpath "$.data.blocks[0].transactions.userCommands[3].kind" == "PAYMENT"
jsonpath "$.data.blocks[0].transactions.userCommands[3].from" == "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy"
jsonpath "$.data.blocks[0].transactions.userCommands[3].to" == "B62qjYanmV7y9njVeH5UHkz3GYBm7xKir1rAnoY4KsEYUGLMiU45FSM"
jsonpath "$.data.blocks[0].transactions.userCommands[3].amount" == 10000
jsonpath "$.data.blocks[0].transactions.userCommands[3].fee" == 50000000
jsonpath "$.data.blocks[0].transactions.userCommands[3].blockHeight" == 3

duration < 2000

# 
# Blocks state hash SNARK jobs query
# 

POST {{url}}
```graphql
{
  blocks(query: {stateHash: "3NLj4yaEa4kGamq93k8ZMqbiZWuR26USzzndXVGoBupEZRQ6zJjm"}) {
    snarkJobs {
      blockHeight
      blockStateHash
      dateTime
      prover
      fee
    }
  }
}
```
HTTP 200
[Asserts]

# Unique block
jsonpath "$.data.blocks" count == 1

# Total SNARKs
jsonpath "$.data.blocks[0].snarkJobs" count == 64

# first SNARK
jsonpath "$.data.blocks[0].snarkJobs[0].blockHeight" == 111
jsonpath "$.data.blocks[0].snarkJobs[0].blockStateHash" == "3NLj4yaEa4kGamq93k8ZMqbiZWuR26USzzndXVGoBupEZRQ6zJjm"
jsonpath "$.data.blocks[0].snarkJobs[0].dateTime" == "2021-03-17T08:06:00.000Z"
jsonpath "$.data.blocks[0].snarkJobs[0].prover" == "B62qrCz3ehCqi8Pn8y3vWC9zYEB9RKsidauv15DeZxhzkxL3bKeba5h"
jsonpath "$.data.blocks[0].snarkJobs[0].fee" == 0

# last SNARK
jsonpath "$.data.blocks[0].snarkJobs[63].blockHeight" == 111
jsonpath "$.data.blocks[0].snarkJobs[63].blockStateHash" == "3NLj4yaEa4kGamq93k8ZMqbiZWuR26USzzndXVGoBupEZRQ6zJjm"
jsonpath "$.data.blocks[0].snarkJobs[63].dateTime" == "2021-03-17T08:06:00.000Z"
jsonpath "$.data.blocks[0].snarkJobs[63].prover" == "B62qrCz3ehCqi8Pn8y3vWC9zYEB9RKsidauv15DeZxhzkxL3bKeba5h"
jsonpath "$.data.blocks[0].snarkJobs[63].fee" == 0

duration < 2000

# 
# Blocks canonical block height bounded ascending query
# 

POST {{url}}
```graphql
query Blocks($limit: Int = 10, $sort_by: BlockSortByInput!, $query: BlockQueryInput!) {
  blocks(limit: $limit, sortBy: $sort_by, query: $query ) {
    stateHash
    blockHeight
    canonical
  }
}

variables {
  "limit": 100,
  "sort_by": "BLOCKHEIGHT_DESC",
  "query": {
    "canonical": true,
    "blockHeight_lte": 50,
    "blockHeight_gt": 10
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.blocks" count == 40

# first datum
jsonpath "$.data.blocks[0].stateHash" == "3NLJeP3pBr2UFgLZpNgwKEw37UeYEsyuP63U9A3Q8D8ATBStWw6Y"
jsonpath "$.data.blocks[0].blockHeight" == 50
jsonpath "$.data.blocks[0].canonical" == true

# last datum
jsonpath "$.data.blocks[39].stateHash" == "3NLMeYAFXxsmhSFtLHFxdtjGcfHTVFmBmBF8uTJvP4Ve5yEmxYeA"
jsonpath "$.data.blocks[39].blockHeight" == 11
jsonpath "$.data.blocks[39].canonical" == true

duration < 2000

# 
# Blocks canonical global slot bounded query
# 

POST {{url}}
```graphql
query Blocks(
  $query: BlockQueryInput!
  $limit: Int = 10
  $sort_by: BlockSortByInput!
) {
  blocks(limit: $limit, query: $query, sortBy: $sort_by ) {
    stateHash
    blockHeight
    canonical
  }
}

variables {
  "limit": 100,
  "sort_by": "BLOCKHEIGHT_ASC",
  "query": {
    "canonical": true,
    "protocolState": {
      "consensusState": {
        "slotSinceGenesis_lte": 100,
        "slotSinceGenesis_gt": 9
      }
    }
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.blocks" count == 62

# first datum
jsonpath "$.data.blocks[0].stateHash" == "3NLMeYAFXxsmhSFtLHFxdtjGcfHTVFmBmBF8uTJvP4Ve5yEmxYeA"
jsonpath "$.data.blocks[0].blockHeight" == 11
jsonpath "$.data.blocks[0].canonical" == true

# last datum
jsonpath "$.data.blocks[61].stateHash" == "3NLjEUEg2888QNjKyKgphTtAdwLV2tmgMycFP3nPFHmy7q9uEwmk"
jsonpath "$.data.blocks[61].blockHeight" == 72
jsonpath "$.data.blocks[61].canonical" == true

duration < 2000

#
# Simple blocks query - https://github.com/Granola-Team/mina-indexer/issues/928
#

POST {{url}}
```graphql
{
  blocks(limit: 10) {
    stateHash
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.blocks" count == 10

duration < 2000

#
# Blocks protocol state query
#

POST {{url}}
```graphql
{
  blocks(query: {canonical: true, protocolState: { consensusState: {slotSinceGenesis : 173}}}) {
    stateHash
    canonical
    protocolState {
      consensusState {
        slotSinceGenesis
      }
    }
  }
}
```
HTTP 200
[Asserts]

jsonpath "$.data.blocks[0].stateHash" == "3NLNyQC4XgQX2Q9H7fC2UxFZKY4xwwUZop8jVR24SWYNNE93FsnS"
jsonpath "$.data.blocks[0].protocolState.consensusState.slotSinceGenesis" == 173
jsonpath "$.data.blocks[0].canonical" == true

duration < 200

#
# Test for early exit condition when filtering use blockHeight_lte
#

POST {{url}}
```graphql
query Blocks($limit: Int = 10, $sort_by: BlockSortByInput!, $query: BlockQueryInput!) {
  blocks(limit: $limit, sortBy: $sort_by, query: $query ) {
    stateHash
  }
}

variables {
  "sort_by": "BLOCKHEIGHT_DESC",
  "query": {
    "canonical": true,
    "blockHeight_lte": 100
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.blocks" count == 10

#
# Test for early exit condition when filtering use blockHeight_lte
#

POST {{url}}
```graphql
query BlocksQuery(
  $query: BlockQueryInput!
  $limit: Int = 10
  $sort_by: BlockSortByInput!
) {
  blocks(query: $query, limit: $limit, sortBy: $sort_by) {
    stateHash
  }
}

variables {
  "limit": 50,
  "sort_by": "BLOCKHEIGHT_DESC",
  "query": {
    "canonical": true,
    "blockHeight_lte": 100
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.blocks" count == 50

#
# Blocks epoch/total counts query
#

POST {{url}}
```graphql
query Blocks {
  blocks(limit: 1) {
    epoch_num_blocks
    epoch_num_canonical_blocks
    epoch_num_supercharged_blocks
    total_num_blocks
    total_num_canonical_blocks
    total_num_supercharged_blocks
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.blocks" count == 1

# data counts
jsonpath "$.data.blocks[0].epoch_num_blocks" == 204
jsonpath "$.data.blocks[0].epoch_num_canonical_blocks" == 120
jsonpath "$.data.blocks[0].epoch_num_supercharged_blocks" == 23

jsonpath "$.data.blocks[0].total_num_blocks" == 204
jsonpath "$.data.blocks[0].total_num_canonical_blocks" == 120
jsonpath "$.data.blocks[0].total_num_supercharged_blocks" == 23

duration < 200

#
# Per block data counts
#

POST {{url}}
```graphql
query Blocks {
  blocks(limit: 10, query: {
    stateHash: "3NL33j16AWm3Jhjj1Ud25E54hu7HpUq4WBQcAiijEKMfXqwFJwzK"
  }) {
    block_num_snarks
    block_num_user_commands
    block_num_internal_commands
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.blocks" count == 1

# data counts
jsonpath "$.data.blocks[0].block_num_snarks" == 64
jsonpath "$.data.blocks[0].block_num_user_commands" == 3
jsonpath "$.data.blocks[0].block_num_internal_commands" == 2

#
# Non-canonical blocks query
#

POST {{url}}
```graphql
{
  blocks(limit: 10, query: {canonical: false}) {
    stateHash
    blockHeight
    canonical
    
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.blocks" count == 10

# heights
jsonpath "$.data.blocks[0].blockHeight" == 120
jsonpath "$.data.blocks[1].blockHeight" == 118
jsonpath "$.data.blocks[2].blockHeight" == 117
jsonpath "$.data.blocks[3].blockHeight" == 117
jsonpath "$.data.blocks[4].blockHeight" == 115
jsonpath "$.data.blocks[5].blockHeight" == 115
jsonpath "$.data.blocks[6].blockHeight" == 113
jsonpath "$.data.blocks[7].blockHeight" == 113
jsonpath "$.data.blocks[8].blockHeight" == 112
jsonpath "$.data.blocks[9].blockHeight" == 111

# canonicity
jsonpath "$.data.blocks[0].canonical" == false
jsonpath "$.data.blocks[1].canonical" == false
jsonpath "$.data.blocks[2].canonical" == false
jsonpath "$.data.blocks[3].canonical" == false
jsonpath "$.data.blocks[4].canonical" == false
jsonpath "$.data.blocks[5].canonical" == false
jsonpath "$.data.blocks[6].canonical" == false
jsonpath "$.data.blocks[7].canonical" == false
jsonpath "$.data.blocks[8].canonical" == false
jsonpath "$.data.blocks[9].canonical" == false

duration < 1000

#
# Unique block producer last n blocks query
#

POST {{url}}
```graphql
{
  blocks(limit: 10, query: {unique_block_producers_last_n_blocks: 10}) {
    num_unique_block_producers_last_n_blocks
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.blocks" count == 1

# only datum
jsonpath "$.data.blocks[0].num_unique_block_producers_last_n_blocks" == 17

duration < 1000

#
# Per epoch number of slots produced
#

POST {{url}}
```graphql
{
  blocks(limit: 1) {
    epoch_num_slots_produced
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.blocks" count == 1

# only datum
jsonpath "$.data.blocks[0].epoch_num_slots_produced" == 120

duration < 1000
