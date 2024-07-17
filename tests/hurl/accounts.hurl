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
jsonpath "$.data.accounts[0].balance" == 720020000000
jsonpath "$.data.accounts[0].nonce" == 0
jsonpath "$.data.accounts[0].delegate" == "B62qj8KB2fk59NkV4VuoTkVXHjw8VJzC3ybKrWo7zuDC9xTiWXPygEe"
jsonpath "$.data.accounts[0].timeLocked" == false

duration < 750

#
# Accounts public key timing query
#

#
# Account public key timing query
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

# total data count
jsonpath "$.data.accounts" count == 1

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

duration < 750

#
# Accounts public key epoch & total blocks produced query
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

duration < 750

#
# Accounts public key epoch & total snarks query
#

POST {{url}}
```graphql
query Accounts {
  accounts(query: {publicKey: "B62qrCz3ehCqi8Pn8y3vWC9zYEB9RKsidauv15DeZxhzkxL3bKeba5h"}) {
    publicKey
    pk_epoch_num_snarks
    pk_total_num_snarks
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.accounts" count == 1

# block production info
jsonpath "$.data.accounts[0].publicKey" == "B62qrCz3ehCqi8Pn8y3vWC9zYEB9RKsidauv15DeZxhzkxL3bKeba5h"
jsonpath "$.data.accounts[0].pk_epoch_num_snarks" == 64
jsonpath "$.data.accounts[0].pk_total_num_snarks" == 64

duration < 750

#
# Accounts public key epoch & total user commands query
#

POST {{url}}
```graphql
query Accounts {
  accounts(query: {publicKey: "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy"}) {
    publicKey
    pk_epoch_num_user_commands
    pk_total_num_user_commands
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.accounts" count == 1

# block production info
jsonpath "$.data.accounts[0].publicKey" == "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy"
jsonpath "$.data.accounts[0].pk_epoch_num_user_commands" == 294
jsonpath "$.data.accounts[0].pk_total_num_user_commands" == 294

duration < 750

#
# Accounts public key epoch & total internal commands query
#

POST {{url}}
```graphql
query Accounts {
  accounts(query: {publicKey: "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy"}) {
    publicKey
    pk_epoch_num_internal_commands
    pk_total_num_internal_commands
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.accounts" count == 1

# block production info
jsonpath "$.data.accounts[0].publicKey" == "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy"
jsonpath "$.data.accounts[0].pk_epoch_num_internal_commands" == 196
jsonpath "$.data.accounts[0].pk_total_num_internal_commands" == 196

duration < 750

#
# Account default (balance descending) query
#


POST {{url}}
```graphql
{
  accounts {
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

jsonpath "$.data.accounts" count == 100

# first datum
jsonpath "$.data.accounts[0].publicKey" == "B62qpbZkvpHZ1a5nsTbANuRtrdw4YraTyA4nvJDm6HpP1YMC9QStxX3"
jsonpath "$.data.accounts[0].balance" == 75000000000000000
jsonpath "$.data.accounts[0].nonce" == 0
jsonpath "$.data.accounts[0].delegate" == "B62qqhURJQo3CvWC3WFo9LhUhtcaJWLBcJsaA3DXaU2GH5KgXujZiwB"
jsonpath "$.data.accounts[0].timeLocked" == false
jsonpath "$.data.accounts[0].timing" == null

# last datum
jsonpath "$.data.accounts[99].publicKey" == "B62qmKJxgh6h4i56hTXmkDL2Xpesm94NNm2Ev4ySSA4rCE2Z3JbfZhc"
jsonpath "$.data.accounts[99].balance" == 1693980637751650
jsonpath "$.data.accounts[99].nonce" == 0
jsonpath "$.data.accounts[99].delegate" == "B62qnvzUAvwnAiK3eMVQooshDA5AmEF9jKRrUTt5cwbCvVFiF47vdqp"
jsonpath "$.data.accounts[99].timeLocked" == true
jsonpath "$.data.accounts[99].timing.initial_minimum_balance" == 1000000000
jsonpath "$.data.accounts[99].timing.cliff_time" == 345600
jsonpath "$.data.accounts[99].timing.cliff_amount" == 1000000000
jsonpath "$.data.accounts[99].timing.vesting_period" == 1
jsonpath "$.data.accounts[99].timing.vesting_increment" == 0

duration < 750

#
# Account balance descending query
#


POST {{url}}
```graphql
{
  accounts(sortBy: "BALANCE_DESC") {
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

jsonpath "$.data.accounts" count == 100

# first datum
jsonpath "$.data.accounts[0].publicKey" == "B62qpbZkvpHZ1a5nsTbANuRtrdw4YraTyA4nvJDm6HpP1YMC9QStxX3"
jsonpath "$.data.accounts[0].balance" == 75000000000000000
jsonpath "$.data.accounts[0].nonce" == 0
jsonpath "$.data.accounts[0].delegate" == "B62qqhURJQo3CvWC3WFo9LhUhtcaJWLBcJsaA3DXaU2GH5KgXujZiwB"
jsonpath "$.data.accounts[0].timeLocked" == false

# last datum
jsonpath "$.data.accounts[99].publicKey" == "B62qmKJxgh6h4i56hTXmkDL2Xpesm94NNm2Ev4ySSA4rCE2Z3JbfZhc"
jsonpath "$.data.accounts[99].balance" == 1693980637751650
jsonpath "$.data.accounts[99].nonce" == 0
jsonpath "$.data.accounts[99].delegate" == "B62qnvzUAvwnAiK3eMVQooshDA5AmEF9jKRrUTt5cwbCvVFiF47vdqp"
jsonpath "$.data.accounts[99].timeLocked" == true
jsonpath "$.data.accounts[99].timing.initial_minimum_balance" == 1000000000
jsonpath "$.data.accounts[99].timing.cliff_time" == 345600
jsonpath "$.data.accounts[99].timing.cliff_amount" == 1000000000
jsonpath "$.data.accounts[99].timing.vesting_period" == 1
jsonpath "$.data.accounts[99].timing.vesting_increment" == 0

duration < 750
