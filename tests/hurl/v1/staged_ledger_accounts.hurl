# 
# State hash query
# 

POST {{url}}
```graphql
{
  stagedLedgerAccounts(
    limit: 100
    sortBy: BALANCE_DESC
    query: { stateHash: "3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ" }
  ) {
    balance
    balance_nano
    delegate
    nonce
    publicKey
    username
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.stagedLedgerAccounts" count == 100

# first datum
jsonpath "$.data.stagedLedgerAccounts[0].balance" == 75000000.0
jsonpath "$.data.stagedLedgerAccounts[0].balance_nano" == 75000000000000000
jsonpath "$.data.stagedLedgerAccounts[0].delegate" == "B62qqhURJQo3CvWC3WFo9LhUhtcaJWLBcJsaA3DXaU2GH5KgXujZiwB"
jsonpath "$.data.stagedLedgerAccounts[0].nonce" == 0
jsonpath "$.data.stagedLedgerAccounts[0].publicKey" == "B62qpbZkvpHZ1a5nsTbANuRtrdw4YraTyA4nvJDm6HpP1YMC9QStxX3"
jsonpath "$.data.stagedLedgerAccounts[0].username" == null

# last datum
jsonpath "$.data.stagedLedgerAccounts[99].balance" == 1693980.63775165
jsonpath "$.data.stagedLedgerAccounts[99].balance_nano" == 1693980637751650
jsonpath "$.data.stagedLedgerAccounts[99].delegate" == "B62qnvzUAvwnAiK3eMVQooshDA5AmEF9jKRrUTt5cwbCvVFiF47vdqp"
jsonpath "$.data.stagedLedgerAccounts[99].nonce" == 0
jsonpath "$.data.stagedLedgerAccounts[99].publicKey" == "B62qmKJxgh6h4i56hTXmkDL2Xpesm94NNm2Ev4ySSA4rCE2Z3JbfZhc"
jsonpath "$.data.stagedLedgerAccounts[99].username" == "O(1) Labs"

duration < 1000

# 
# Ledger hash query
# 

POST {{url}}
```graphql
{
  stagedLedgerAccounts(
    limit: 100
    sortBy: BALANCE_DESC
    query: { ledgerHash: "jx7buQVWFLsXTtzRgSxbYcT8EYLS8KCZbLrfDcJxMtyy4thw2Ee" }
  ) {
    balance
    balance_nano
    delegate
    nonce
    publicKey
    username
  }
}
```
HTTP 200
[Asserts]

# Total accounts returned
jsonpath "$.data.stagedLedgerAccounts" count == 100

jsonpath "$.data.stagedLedgerAccounts[0].balance" == 75000000.0
jsonpath "$.data.stagedLedgerAccounts[0].balance_nano" == 75000000000000000
jsonpath "$.data.stagedLedgerAccounts[0].delegate" == "B62qqhURJQo3CvWC3WFo9LhUhtcaJWLBcJsaA3DXaU2GH5KgXujZiwB"
jsonpath "$.data.stagedLedgerAccounts[0].nonce" == 0
jsonpath "$.data.stagedLedgerAccounts[0].publicKey" == "B62qpbZkvpHZ1a5nsTbANuRtrdw4YraTyA4nvJDm6HpP1YMC9QStxX3"
jsonpath "$.data.stagedLedgerAccounts[0].username" == null

jsonpath "$.data.stagedLedgerAccounts[99].balance" == 1693980.63775165
jsonpath "$.data.stagedLedgerAccounts[99].balance_nano" == 1693980637751650
jsonpath "$.data.stagedLedgerAccounts[99].delegate" == "B62qnvzUAvwnAiK3eMVQooshDA5AmEF9jKRrUTt5cwbCvVFiF47vdqp"
jsonpath "$.data.stagedLedgerAccounts[99].nonce" == 0
jsonpath "$.data.stagedLedgerAccounts[99].publicKey" == "B62qmKJxgh6h4i56hTXmkDL2Xpesm94NNm2Ev4ySSA4rCE2Z3JbfZhc"
jsonpath "$.data.stagedLedgerAccounts[99].username" == "O(1) Labs"

duration < 1000

# 
# Public key & state hash query
# 

POST {{url}}
```graphql
{
  stagedLedgerAccounts(
    limit: 10
    query: {
      publicKey: "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy"
      stateHash: "3NKkJDmNZGYdKVDDJkkamGdvNzASia2SXxKpu18imps7KqbNXENY"
    }
  ) {
    balance
    balance_nano
    nonce
    publicKey
  }
}
```
HTTP 200
[Asserts]

# account
jsonpath "$.data.stagedLedgerAccounts" count == 1
jsonpath "$.data.stagedLedgerAccounts[0].balance" == 1438.69
jsonpath "$.data.stagedLedgerAccounts[0].balance_nano" == 1438690000000
jsonpath "$.data.stagedLedgerAccounts[0].publicKey" == "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy"
jsonpath "$.data.stagedLedgerAccounts[0].nonce" == 15

duration < 1000

# 
# Public key & block height query
# 

POST {{url}}
```graphql
{
  stagedLedgerAccounts(
    limit: 10
    query: {
      publicKey: "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy"
      blockchain_length: 12
    }
  ) {
    balance
    balance_nano
    nonce
    publicKey
  }
}
```
HTTP 200
[Asserts]

# account
jsonpath "$.data.stagedLedgerAccounts" count == 1
jsonpath "$.data.stagedLedgerAccounts[0].balance" == 1438.69
jsonpath "$.data.stagedLedgerAccounts[0].balance_nano" == 1438690000000
jsonpath "$.data.stagedLedgerAccounts[0].publicKey" == "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy"
jsonpath "$.data.stagedLedgerAccounts[0].nonce" == 15

duration < 1000

# 
# Public key & staged ledger hash query
# 

POST {{url}}
```graphql
{
  stagedLedgerAccounts(
    limit: 10
    query: {
      publicKey: "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy"
      ledgerHash: "jwVZUkFNgb5fBf6ZRBtYXrwWs32erEyPTLXzdduGNhH6F71wjan"
    }
  ) {
    balance
    balance_nano
    nonce
    publicKey
  }
}
```
HTTP 200
[Asserts]

# account
jsonpath "$.data.stagedLedgerAccounts" count == 1
jsonpath "$.data.stagedLedgerAccounts[0].balance" == 1438.69
jsonpath "$.data.stagedLedgerAccounts[0].balance_nano" == 1438690000000
jsonpath "$.data.stagedLedgerAccounts[0].publicKey" == "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy"
jsonpath "$.data.stagedLedgerAccounts[0].nonce" == 15

duration < 1000

# 
# Missing public key query
# 

POST {{url}}
```graphql
{
  stagedLedgerAccounts(
    limit: 10
    query: {
      publicKey: "B62qjHdYUPTHQkwDWUbDYscteT2LFj3ro1vz9fnxMyHTACe6C2fLbSd"
      ledgerHash: "jwVZUkFNgb5fBf6ZRBtYXrwWs32erEyPTLXzdduGNhH6F71wjan"
    }
  ) {
    balance_nano
  }
}
```
HTTP 200
[Asserts]

# account
jsonpath "$.data.stagedLedgerAccounts" == null

duration < 1000
