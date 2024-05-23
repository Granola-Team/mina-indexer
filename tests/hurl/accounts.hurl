#
# Accounts public key query
#

POST {{url}}
```graphql
query Accounts {
  accounts(query: {publicKey: "B62qj8KB2fk59NkV4VuoTkVXHjw8VJzC3ybKrWo7zuDC9xTiWXPygEe" }) {
    publicKey
    balance
    username
    nonce
    delegate
    timeLocked
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.accounts" count == 1

# only datum
jsonpath "$.data.accounts[0].publicKey" == "B62qj8KB2fk59NkV4VuoTkVXHjw8VJzC3ybKrWo7zuDC9xTiWXPygEe"
jsonpath "$.data.accounts[0].balance" == 719020000000
jsonpath "$.data.accounts[0].nonce" == 1
jsonpath "$.data.accounts[0].delegate" == "B62qj8KB2fk59NkV4VuoTkVXHjw8VJzC3ybKrWo7zuDC9xTiWXPygEe"
jsonpath "$.data.accounts[0].timeLocked" == false

duration < 200

#
# Accounts public key timing query
#

POST {{url}}
```graphql
query Accounts {
  accounts(query: {publicKey: "B62qoTaYnDYrtmver4jnkgJbFU6iohjtALTU4ebf4yCKrxFFHd9Dimh" }) {
    publicKey
    balance
    username
    nonce
    delegate
    timeLocked
    timing {
      initial_minimum_balance
      cliff_time
      cliff_amount
      vesting_period
      vesting_increment
    }
  }
}
```
HTTP 200
[Asserts]

# account info
jsonpath "$.data.accounts[0].publicKey" == "B62qoTaYnDYrtmver4jnkgJbFU6iohjtALTU4ebf4yCKrxFFHd9Dimh"
jsonpath "$.data.accounts[0].balance" == 502777775000000
jsonpath "$.data.accounts[0].username" == "Unknown"
jsonpath "$.data.accounts[0].nonce" == 0
jsonpath "$.data.accounts[0].delegate" == "B62qj8KB2fk59NkV4VuoTkVXHjw8VJzC3ybKrWo7zuDC9xTiWXPygEe"

# timing info
jsonpath "$.data.accounts[0].timeLocked" == true
jsonpath "$.data.accounts[0].timing.initial_minimum_balance" == 1000000000
jsonpath "$.data.accounts[0].timing.cliff_time" == 691200
jsonpath "$.data.accounts[0].timing.cliff_amount" == 1000000000
jsonpath "$.data.accounts[0].timing.vesting_period" == 1
jsonpath "$.data.accounts[0].timing.vesting_increment" == 0

duration < 200

#
# Accounts public key epoch blocks produced query
#

POST {{url}}
```graphql
query Accounts {
  accounts(query: {publicKey: "B62qqE5R5pJDUjPrKZMtTkPKUPL27kwNZ1sHkZaowxSLosZvLudt3kW"}) {
    publicKey
    pk_epoch_num_blocks
    pk_total_num_blocks
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.accounts" count == 1

# block production info
jsonpath "$.data.accounts[0].publicKey" == "B62qqE5R5pJDUjPrKZMtTkPKUPL27kwNZ1sHkZaowxSLosZvLudt3kW"
jsonpath "$.data.accounts[0].pk_epoch_num_blocks" == 5
jsonpath "$.data.accounts[0].pk_total_num_blocks" == 5

duration < 200
