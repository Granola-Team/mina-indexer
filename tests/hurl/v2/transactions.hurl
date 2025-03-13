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


#
# Transaction data
#

POST {{url}}
```graphql
{
  transactions(limit: 1) {
    blockHeight
    canonical
    hash
    epoch_num_user_commands
    total_num_user_commands
    epoch_num_zkapp_commands
    total_num_zkapp_commands
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.transactions" count == 1

# only datum
jsonpath "$.data.transactions[0].blockHeight" == 359617
jsonpath "$.data.transactions[0].canonical" == true
jsonpath "$.data.transactions[0].hash" == "5JvJ6fA7cL6SoVjKYg6T1W5f6PKWbt9tevpeuVbWBBuKktaL8aJM"
jsonpath "$.data.transactions[0].epoch_num_user_commands" == 155
jsonpath "$.data.transactions[0].total_num_user_commands" == 155
jsonpath "$.data.transactions[0].epoch_num_zkapp_commands" == 80
jsonpath "$.data.transactions[0].total_num_zkapp_commands" == 80

duration < 100

#
# Filter based on token ID
#

POST {{url}}
```graphql
{
  transactions(
    limit: 1
    query: {
      canonical: true,
      zkapp: true,
      token: "wfG3GivPMttpt6nQnPuX9eDPnoyA5RJZY23LTc4kkNkCRH2gUd"
    }
  ) {
    tokens
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.transactions" count == 1

# only datum
jsonpath "$.data.transactions[0].tokens" count == 2
jsonpath "$.data.transactions[0].tokens[0]" == "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf"
jsonpath "$.data.transactions[0].tokens[1]" == "wfG3GivPMttpt6nQnPuX9eDPnoyA5RJZY23LTc4kkNkCRH2gUd"

duration < 100
