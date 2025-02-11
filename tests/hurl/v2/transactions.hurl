#
# Zkapp transactions canonical block height ascending query
#

POST {{url}}
```graphql
{
  transactions(limit: 50, sortBy: BLOCKHEIGHT_ASC, query: { canonical: true, zkapp: true }) {
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
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.transactions" count == 42

# first datum
jsonpath "$.data.transactions[0].blockHeight" == 359609
jsonpath "$.data.transactions[0].canonical" == true
jsonpath "$.data.transactions[0].memo" == ""
jsonpath "$.data.transactions[0].hash" == "5Jtarnuf2gvYuQsv3HYeDYNLWMb3ZosKubkYYMDxWhaYxojbaDMg"

# last datum
jsonpath "$.data.transactions[41].blockHeight" == 359617
jsonpath "$.data.transactions[41].canonical" == true
jsonpath "$.data.transactions[41].memo" == ""
jsonpath "$.data.transactions[41].hash" == "5Jv8ZMpRwgxuDkXp4fDqRpCSQ1s7k8shGUTMxyPQbS3UJiJvjb3U"

duration < 100

#
# Zkapp transactions canonical block height descending query
#

POST {{url}}
```graphql
{
  transactions(
    limit: 25
    sortBy: BLOCKHEIGHT_DESC
    query: {
      canonical: true,
      zkapp: true
    }
  ) {
    blockHeight
    canonical
    hash
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.transactions" count == 25

# first datum
jsonpath "$.data.transactions[0].blockHeight" == 359617
jsonpath "$.data.transactions[0].canonical" == true
jsonpath "$.data.transactions[0].hash" == "5Jv8ZMpRwgxuDkXp4fDqRpCSQ1s7k8shGUTMxyPQbS3UJiJvjb3U"

# last datum
jsonpath "$.data.transactions[24].blockHeight" == 359613
jsonpath "$.data.transactions[24].canonical" == true
jsonpath "$.data.transactions[24].hash" == "5Jtef4ue1udQiniSy2co1LZK3wvoQCAZV6595VSXt4znqq57djpz"

duration < 100
