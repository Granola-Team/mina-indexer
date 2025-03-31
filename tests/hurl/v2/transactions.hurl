#
# Zkapp transactions canonical block height ascending query
#

POST {{url}}
```graphql
{
  transactions(
    limit: 50
    sortBy: BLOCKHEIGHT_ASC
    query: { canonical: true, zkapp: true }
  ) {
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
jsonpath "$.data.transactions[0].memo" == "Test ZKApp to Receiver"
jsonpath "$.data.transactions[0].hash" == "5Jtarnuf2gvYuQsv3HYeDYNLWMb3ZosKubkYYMDxWhaYxojbaDMg"

# last datum
jsonpath "$.data.transactions[41].blockHeight" == 359617
jsonpath "$.data.transactions[41].canonical" == true
jsonpath "$.data.transactions[41].memo" == "Test ZKApp to Receiver"
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
    query: { canonical: true, zkapp: true }
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
      canonical: true
      zkapp: true
      token: "wfG3GivPMttpt6nQnPuX9eDPnoyA5RJZY23LTc4kkNkCRH2gUd"
    }
  ) {
    blockHeight
    tokens
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.transactions" count == 1

# only datum
jsonpath "$.data.transactions[0].blockHeight" == 359617

# tokens
jsonpath "$.data.transactions[0].tokens" count == 2
jsonpath "$.data.transactions[0].tokens[0]" == "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf"
jsonpath "$.data.transactions[0].tokens[1]" == "wfG3GivPMttpt6nQnPuX9eDPnoyA5RJZY23LTc4kkNkCRH2gUd"

duration < 100

#
# Accounts updated query
#

POST {{url}}
```graphql
{
  transactions(limit: 1, query: { canonical: true, zkapp: true }) {
    zkapp {
      accounts_updated {
        pk
        token
        balance_change
        increment_nonce
      }
      actions
      events
    }
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.transactions" count == 1

# only datum
jsonpath "$.data.transactions[0].zkapp.actions" count == 0
jsonpath "$.data.transactions[0].zkapp.events" count == 0
jsonpath "$.data.transactions[0].zkapp.accounts_updated" count == 2

# account updates
jsonpath "$.data.transactions[0].zkapp.accounts_updated[0].pk" == "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5"
jsonpath "$.data.transactions[0].zkapp.accounts_updated[0].token" == "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf"
jsonpath "$.data.transactions[0].zkapp.accounts_updated[0].balance_change" == -2000000000
jsonpath "$.data.transactions[0].zkapp.accounts_updated[0].increment_nonce" == false

jsonpath "$.data.transactions[0].zkapp.accounts_updated[1].pk" == "B62qn4SxXSBZuCUCKH3ZqgP32eab9bKNrEXkjoczEnerihQrSNnxoc5"
jsonpath "$.data.transactions[0].zkapp.accounts_updated[1].token" == "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf"
jsonpath "$.data.transactions[0].zkapp.accounts_updated[1].balance_change" == 2000000000
jsonpath "$.data.transactions[0].zkapp.accounts_updated[1].increment_nonce" == false

duration < 100

#
# Zkapp account update receiver query
#

POST {{url}}
```graphql
{
  transactions(
    query: {
      canonical: true
      to: "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF"
    }
  ) {
    blockHeight
    amount
    fee
    from
    nonce
    zkapp {
      accounts_updated {
        pk
        token
        balance_change
        increment_nonce
        symbol
      }
    }
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.transactions" count == 1

# only datum
jsonpath "$.data.transactions[0].blockHeight" == 359617
jsonpath "$.data.transactions[0].amount" == 0
jsonpath "$.data.transactions[0].fee" == 100000000
jsonpath "$.data.transactions[0].from" == "B62qoxZPhqRsKromMF72kjZr6LQnufZ8T2iZuDzCmtuDnnddCRF7fpp"
jsonpath "$.data.transactions[0].nonce" == 3

# account updates
jsonpath "$.data.transactions[0].zkapp.accounts_updated" count == 4

jsonpath "$.data.transactions[0].zkapp.accounts_updated[0].pk" == "B62qoxZPhqRsKromMF72kjZr6LQnufZ8T2iZuDzCmtuDnnddCRF7fpp"
jsonpath "$.data.transactions[0].zkapp.accounts_updated[0].token" == "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf"
jsonpath "$.data.transactions[0].zkapp.accounts_updated[0].balance_change" == -2000000000
jsonpath "$.data.transactions[0].zkapp.accounts_updated[0].increment_nonce" == false
jsonpath "$.data.transactions[0].zkapp.accounts_updated[0].symbol" == "MINA"

jsonpath "$.data.transactions[0].zkapp.accounts_updated[1].pk" == "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF"
jsonpath "$.data.transactions[0].zkapp.accounts_updated[1].token" == "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf"
jsonpath "$.data.transactions[0].zkapp.accounts_updated[1].balance_change" == 0
jsonpath "$.data.transactions[0].zkapp.accounts_updated[1].increment_nonce" == true
jsonpath "$.data.transactions[0].zkapp.accounts_updated[1].symbol" == "MINA"

jsonpath "$.data.transactions[0].zkapp.accounts_updated[2].pk" == "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF"
jsonpath "$.data.transactions[0].zkapp.accounts_updated[2].token" == "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf"
jsonpath "$.data.transactions[0].zkapp.accounts_updated[2].balance_change" == 0
jsonpath "$.data.transactions[0].zkapp.accounts_updated[2].increment_nonce" == false
jsonpath "$.data.transactions[0].zkapp.accounts_updated[2].symbol" == "MINA"

jsonpath "$.data.transactions[0].zkapp.accounts_updated[3].pk" == "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF"
jsonpath "$.data.transactions[0].zkapp.accounts_updated[3].token" == "wfG3GivPMttpt6nQnPuX9eDPnoyA5RJZY23LTc4kkNkCRH2gUd"
jsonpath "$.data.transactions[0].zkapp.accounts_updated[3].balance_change" == 100000000000000
jsonpath "$.data.transactions[0].zkapp.accounts_updated[3].increment_nonce" == false
jsonpath "$.data.transactions[0].zkapp.accounts_updated[3].symbol" == "MINU"

duration < 100