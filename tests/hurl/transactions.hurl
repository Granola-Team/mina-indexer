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
jsonpath "$.data.transactions[0].memo" == "E4YM2vTHhWEg66xpj52JErHUBU4pZ1yageL4TVDDpTTSsv8mK6YaH"
jsonpath "$.data.transactions[0].hash" == "CkpYeG32dVJUjs6iq3oroXWitXar1eBtV3GVFyH5agw7HPp9bG4yQ"

# last datum
jsonpath "$.data.transactions[49].blockHeight" == 59
jsonpath "$.data.transactions[49].canonical" == true
jsonpath "$.data.transactions[49].memo" == "E4YM2vTHhWEg66xpj52JErHUBU4pZ1yageL4TVDDpTTSsv8mK6YaH"
jsonpath "$.data.transactions[49].hash" == "CkpZ3EzeQLT4m3mLUwyKFdvF3cjvvFGUzR9jug14E8bFryTpTsxYz"

duration < 200

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
jsonpath "$.data.transactions[0].memo" == "E4YM2vTHhWEg66xpj52JErHUBU4pZ1yageL4TVDDpTTSsv8mK6YaH"
jsonpath "$.data.transactions[0].hash" == "CkpZreaWRNr1eANhVYLmi8vzRrwkoEwdNyk2FyUa7M4ZQVnA752wL"

# last datum
jsonpath "$.data.transactions[49].blockHeight" == 56
jsonpath "$.data.transactions[49].canonical" == true
jsonpath "$.data.transactions[49].memo" == "E4YM2vTHhWEg66xpj52JErHUBU4pZ1yageL4TVDDpTTSsv8mK6YaH"
jsonpath "$.data.transactions[49].hash" == "CkpZDjNVLadk61ZhYnh9eaLc9g9yx19KqmLjJiZRqtavHsnvsKsnC"

duration < 200

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
jsonpath "$.data.transactions[0].hash" == "CkpZE6rHwLM8iJwBU1ssm1tdMzx9rdyDEhpGU8aCAionzF1k82d5N"
jsonpath "$.data.transactions[0].from" == "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy"
jsonpath "$.data.transactions[0].to" == "B62qjYanmV7y9njVeH5UHkz3GYBm7xKir1rAnoY4KsEYUGLMiU45FSM"
jsonpath "$.data.transactions[0].amount" == 1000000000
jsonpath "$.data.transactions[0].fee" == 10000000

# last datum
jsonpath "$.data.transactions[1].blockHeight" == 11
jsonpath "$.data.transactions[1].hash" == "CkpZwTdKUDFM8Nsq6noJ4pP6esqP7FeJaHiisGrXSqHYvidfvukDA"
jsonpath "$.data.transactions[1].from" == "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy"
jsonpath "$.data.transactions[1].to" == "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy"
jsonpath "$.data.transactions[1].amount" == 1000
jsonpath "$.data.transactions[1].fee" == 10000000

duration < 100

# TODO state hash query
# TODO global slot query

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
jsonpath "$.data.transactions[49].blockHeight" == 56
jsonpath "$.data.transactions[49].canonical" == true
jsonpath "$.data.transactions[49].hash" == "CkpZDjNVLadk61ZhYnh9eaLc9g9yx19KqmLjJiZRqtavHsnvsKsnC"
jsonpath "$.data.transactions[49].from" == "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy"
jsonpath "$.data.transactions[49].to" == "B62qjYanmV7y9njVeH5UHkz3GYBm7xKir1rAnoY4KsEYUGLMiU45FSM"
jsonpath "$.data.transactions[49].amount" == 1000
jsonpath "$.data.transactions[49].fee" == 10000000

duration < 200

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
jsonpath "$.data.transactions[49].blockHeight" == 56
jsonpath "$.data.transactions[49].canonical" == true
jsonpath "$.data.transactions[49].hash" == "CkpZDjNVLadk61ZhYnh9eaLc9g9yx19KqmLjJiZRqtavHsnvsKsnC"
jsonpath "$.data.transactions[49].from" == "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy"
jsonpath "$.data.transactions[49].to" == "B62qjYanmV7y9njVeH5UHkz3GYBm7xKir1rAnoY4KsEYUGLMiU45FSM"
jsonpath "$.data.transactions[49].amount" == 1000
jsonpath "$.data.transactions[49].fee" == 10000000

duration < 200
