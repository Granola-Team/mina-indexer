#
# Transactions canonical block height ascending query
#

POST {{url}}
```graphql
query Transactions($limit: Int, $sort_by: TransactionSortByInput!, $query: TransactionQueryInput!) {
  transactions(limit: $limit, sortBy: $sort_by, query: $query ) {
    blockHeight
    canonical
    amount
    fee
    kind
    to
    from
    nonce
    memo
    hash
    block {
      dateTime
      stateHash
    }
    receiver {
      publicKey
    }
  }
}

variables {
  "limit": 50,
  "sort_by": "BLOCKHEIGHT_ASC",
  "query": {
    "canonical": true
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.transactions" count == 50

# first datum
jsonpath "$.data.transactions[0].blockHeight" == 3
jsonpath "$.data.transactions[0].canonical" == true
jsonpath "$.data.transactions[0].memo" == ""
jsonpath "$.data.transactions[0].hash" == "CkpYeG32dVJUjs6iq3oroXWitXar1eBtV3GVFyH5agw7HPp9bG4yQ"

# last datum
jsonpath "$.data.transactions[49].blockHeight" == 37
jsonpath "$.data.transactions[49].canonical" == true
jsonpath "$.data.transactions[49].memo" == ""
jsonpath "$.data.transactions[49].hash" == "CkpYsbai6qHEoF53e8pFHdiWqhT34sPhSgbUVdxLmfzAZ8oMpKDuK"

duration < 750

#
# Transactions canonical block height descending query
#

POST {{url}}
```graphql
query Transactions($limit: Int, $sort_by: TransactionSortByInput!, $query: TransactionQueryInput!) {
  transactions(limit: $limit, sortBy: $sort_by, query: $query ) {
    blockHeight
    canonical
    memo
    hash
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

# total data count
jsonpath "$.data.transactions" count == 50

# first datum
jsonpath "$.data.transactions[0].blockHeight" == 120
jsonpath "$.data.transactions[0].canonical" == true
jsonpath "$.data.transactions[0].memo" == ""
jsonpath "$.data.transactions[0].hash" == "CkpZreaWRNr1eANhVYLmi8vzRrwkoEwdNyk2FyUa7M4ZQVnA752wL"

# last datum
jsonpath "$.data.transactions[49].blockHeight" == 89
jsonpath "$.data.transactions[49].canonical" == true
jsonpath "$.data.transactions[49].memo" == ""
jsonpath "$.data.transactions[49].hash" == "CkpaJb8GcyrbGYjGeC7aUtwDq3m6yxXSBFhuUVy4Jm8FVvaimbioM"

duration < 750

#
# Transactions canonical specific block height query
#

POST {{url}}
```graphql
query Transactions($limit: Int, $sort_by: TransactionSortByInput!, $query: TransactionQueryInput!) {
  transactions(limit: $limit, sortBy: $sort_by, query: $query ) {
    blockHeight
    canonical
    amount
    fee
    kind
    to
    from
    nonce
    memo
    hash
    block {
      dateTime
      stateHash
    }
    receiver {
      publicKey
    }
  }
}

variables {
  "limit": 50,
  "sort_by": "BLOCKHEIGHT_DESC",
  "query": {
    "blockHeight": 11
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.transactions" count == 2

# first datum
jsonpath "$.data.transactions[0].blockHeight" == 11
jsonpath "$.data.transactions[0].hash" == "CkpZwTdKUDFM8Nsq6noJ4pP6esqP7FeJaHiisGrXSqHYvidfvukDA"
jsonpath "$.data.transactions[0].from" == "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy"
jsonpath "$.data.transactions[0].to" == "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy"
jsonpath "$.data.transactions[0].amount" == 1000
jsonpath "$.data.transactions[0].fee" == 10000000

# last datum
jsonpath "$.data.transactions[1].blockHeight" == 11
jsonpath "$.data.transactions[1].hash" == "CkpZE6rHwLM8iJwBU1ssm1tdMzx9rdyDEhpGU8aCAionzF1k82d5N"
jsonpath "$.data.transactions[1].from" == "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy"
jsonpath "$.data.transactions[1].to" == "B62qjYanmV7y9njVeH5UHkz3GYBm7xKir1rAnoY4KsEYUGLMiU45FSM"
jsonpath "$.data.transactions[1].amount" == 1000000000
jsonpath "$.data.transactions[1].fee" == 10000000

duration < 750

#
# Transactions from (sender) query
#

POST {{url}}
```graphql
query Transactions($limit: Int, $sort_by: TransactionSortByInput!, $query: TransactionQueryInput!) {
  transactions(limit: $limit, sortBy: $sort_by, query: $query ) {
    blockHeight
    canonical
    from
    to
    hash
    amount
    fee
  }
}

variables {
  "limit": 50,
  "sort_by": "BLOCKHEIGHT_DESC",
  "query": {
    "from": "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy",
    "canonical": true
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.transactions" count == 50

# first datum
jsonpath "$.data.transactions[0].blockHeight" == 120
jsonpath "$.data.transactions[0].canonical" == true
jsonpath "$.data.transactions[0].hash" == "CkpZreaWRNr1eANhVYLmi8vzRrwkoEwdNyk2FyUa7M4ZQVnA752wL"
jsonpath "$.data.transactions[0].from" == "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy"
jsonpath "$.data.transactions[0].to" == "B62qjYanmV7y9njVeH5UHkz3GYBm7xKir1rAnoY4KsEYUGLMiU45FSM"
jsonpath "$.data.transactions[0].amount" == 1000
jsonpath "$.data.transactions[0].fee" == 10000000

# last datum
jsonpath "$.data.transactions[49].blockHeight" == 89
jsonpath "$.data.transactions[49].canonical" == true
jsonpath "$.data.transactions[49].hash" == "CkpYpkePDRfsrWHxpGMMxVfWMy5xnSCjYTcsqyAqgo7keHwWThb8N"
jsonpath "$.data.transactions[49].from" == "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy"
jsonpath "$.data.transactions[49].to" == "B62qjYanmV7y9njVeH5UHkz3GYBm7xKir1rAnoY4KsEYUGLMiU45FSM"
jsonpath "$.data.transactions[49].amount" == 1000
jsonpath "$.data.transactions[49].fee" == 10000000

duration < 750

#
# Transactions to (receiver) query
#

POST {{url}}
```graphql
query Transactions($limit: Int, $sort_by: TransactionSortByInput!, $query: TransactionQueryInput!) {
  transactions(limit: $limit, sortBy: $sort_by, query: $query ) {
    blockHeight
    canonical
    from
    to
    hash
    amount
    fee
  }
}

variables {
  "limit": 50,
  "sort_by": "BLOCKHEIGHT_DESC",
  "query": {
    "to": "B62qjYanmV7y9njVeH5UHkz3GYBm7xKir1rAnoY4KsEYUGLMiU45FSM",
    "canonical": true
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.transactions" count == 50

# first datum
jsonpath "$.data.transactions[0].blockHeight" == 120
jsonpath "$.data.transactions[0].canonical" == true
jsonpath "$.data.transactions[0].hash" == "CkpZreaWRNr1eANhVYLmi8vzRrwkoEwdNyk2FyUa7M4ZQVnA752wL"
jsonpath "$.data.transactions[0].from" == "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy"
jsonpath "$.data.transactions[0].to" == "B62qjYanmV7y9njVeH5UHkz3GYBm7xKir1rAnoY4KsEYUGLMiU45FSM"
jsonpath "$.data.transactions[0].amount" == 1000
jsonpath "$.data.transactions[0].fee" == 10000000

# last datum
jsonpath "$.data.transactions[49].blockHeight" == 89
jsonpath "$.data.transactions[49].canonical" == true
jsonpath "$.data.transactions[49].hash" == "CkpYpkePDRfsrWHxpGMMxVfWMy5xnSCjYTcsqyAqgo7keHwWThb8N"
jsonpath "$.data.transactions[49].from" == "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy"
jsonpath "$.data.transactions[49].to" == "B62qjYanmV7y9njVeH5UHkz3GYBm7xKir1rAnoY4KsEYUGLMiU45FSM"
jsonpath "$.data.transactions[49].amount" == 1000
jsonpath "$.data.transactions[49].fee" == 10000000

duration < 750

POST {{url}}
```graphql
query TransactionsQuery(
  $limit: Int = 10
  $sort_by: TransactionSortByInput!
  $query: TransactionQueryInput!
) {
  transactions(limit: $limit, sortBy: $sort_by, query: $query) {
    blockHeight
    failureReason
    canonical
    amount
    fee
    kind
    to
    from
    nonce
    memo
    hash
    block {
      dateTime
      stateHash
    }
    receiver {
      publicKey
    }
  }
}

variables {
  "limit": 100,
  "sort_by": "BLOCKHEIGHT_ASC",
  "query": {
    "blockHeight_lte": 100,
    "canonical": true
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.transactions" count == 100


POST {{url}}
```graphql
query TransactionsQuery(
  $limit: Int = 10
  $sort_by: TransactionSortByInput!
  $query: TransactionQueryInput!
) {
  transactions(limit: $limit, sortBy: $sort_by, query: $query) {
    hash
    blockHeight
    canonical
    block {
      stateHash
    }
  }
}

variables {
  "limit": 1,
  "sort_by": "BLOCKHEIGHT_DESC",
  "query": {
      "canonical": true,
      "block": {
          "stateHash": "3NLNyQC4XgQX2Q9H7fC2UxFZKY4xwwUZop8jVR24SWYNNE93FsnS"
      }
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.transactions" count == 1

jsonpath "$.data.transactions[0].blockHeight" == 120
jsonpath "$.data.transactions[0].canonical" == true
jsonpath "$.data.transactions[0].hash" == "CkpZreaWRNr1eANhVYLmi8vzRrwkoEwdNyk2FyUa7M4ZQVnA752wL"
jsonpath "$.data.transactions[0].block.stateHash" == "3NLNyQC4XgQX2Q9H7fC2UxFZKY4xwwUZop8jVR24SWYNNE93FsnS"

duration < 750

POST {{url}}
```graphql
query TransactionsQueryDuplicate(
  $limit: Int = 10
  $sort_by: TransactionSortByInput!
  $query: TransactionQueryInput!
) {
  transactions(limit: $limit, sortBy: $sort_by, query: $query) {
    blockHeight
    failureReason
    canonical
    amount
    fee
    kind
    to
    from
    nonce
    memo
    hash
    block {
      dateTime
      stateHash
    }
    receiver {
      publicKey
    }
  }
}

variables {
  "limit": 100,
  "sort_by": "BLOCKHEIGHT_DESC",
  "query": {
    "from": "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy",
    "canonical": true
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.transactions" count == 100

jsonpath "$.data.transactions[0].blockHeight" == 120
jsonpath "$.data.transactions[0].canonical" == true
jsonpath "$.data.transactions[0].hash" == "CkpZreaWRNr1eANhVYLmi8vzRrwkoEwdNyk2FyUa7M4ZQVnA752wL"
jsonpath "$.data.transactions[0].block.stateHash" == "3NLNyQC4XgQX2Q9H7fC2UxFZKY4xwwUZop8jVR24SWYNNE93FsnS"
jsonpath "$.data.transactions[0].from" == "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy"

jsonpath "$.data.transactions[99].blockHeight" == 57
jsonpath "$.data.transactions[99].canonical" == true
jsonpath "$.data.transactions[99].hash" == "CkpZCfVEHKrSUq7iV5pz6pAe6achsNCWtSY5G6vMqDgGoay2Ey2i2"
jsonpath "$.data.transactions[99].block.stateHash" == "3NLuaJ8A7ubV4vFtc6YtnPUaMpiKAks2d7HmXByH6NkRS8EKG9Q8"
jsonpath "$.data.transactions[99].from" == "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy"

#
# Transactions by hash (block order)
#

POST {{url}}
```graphql
{
  transactions(query: {hash: "CkpZrxGwdxH2CL8wGTF3B9BgJcc5xyfiF8iyzNvTYMALccvmNcNSN"} ) {
    blockHeight
    canonical
    hash
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.transactions" count == 4

# all data
jsonpath "$.data.transactions[0].blockHeight" == 6
jsonpath "$.data.transactions[0].canonical" == true
jsonpath "$.data.transactions[0].hash" == "CkpZrxGwdxH2CL8wGTF3B9BgJcc5xyfiF8iyzNvTYMALccvmNcNSN"

jsonpath "$.data.transactions[1].blockHeight" == 6
jsonpath "$.data.transactions[1].canonical" == false
jsonpath "$.data.transactions[1].hash" == "CkpZrxGwdxH2CL8wGTF3B9BgJcc5xyfiF8iyzNvTYMALccvmNcNSN"

jsonpath "$.data.transactions[2].blockHeight" == 6
jsonpath "$.data.transactions[2].canonical" == false
jsonpath "$.data.transactions[2].hash" == "CkpZrxGwdxH2CL8wGTF3B9BgJcc5xyfiF8iyzNvTYMALccvmNcNSN"

jsonpath "$.data.transactions[3].blockHeight" == 6
jsonpath "$.data.transactions[3].canonical" == false
jsonpath "$.data.transactions[3].hash" == "CkpZrxGwdxH2CL8wGTF3B9BgJcc5xyfiF8iyzNvTYMALccvmNcNSN"

duration < 750
