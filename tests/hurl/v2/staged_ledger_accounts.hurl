# 
# State hash query
# 

POST {{url}}
```graphql
{
  stagedLedgerAccounts(
    limit: 10
    sortBy: BALANCE_DESC
    query: { stateHash: "3NK4BpDSekaqsG6tx8Nse2zJchRft2JpnbvMiog55WCr5xJZaKeP" }
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
jsonpath "$.data.stagedLedgerAccounts" count == 10

# first datum
jsonpath "$.data.stagedLedgerAccounts[0].balance" == 144390363.9093
jsonpath "$.data.stagedLedgerAccounts[0].balance_nano" == 144390363909300000
jsonpath "$.data.stagedLedgerAccounts[0].delegate" == "B62qrQKS9ghd91shs73TCmBJRW9GzvTJK443DPx2YbqcyoLc56g1ny9"
jsonpath "$.data.stagedLedgerAccounts[0].nonce" == 17
jsonpath "$.data.stagedLedgerAccounts[0].publicKey" == "B62qrQKS9ghd91shs73TCmBJRW9GzvTJK443DPx2YbqcyoLc56g1ny9"
jsonpath "$.data.stagedLedgerAccounts[0].username" == null

# last datum
jsonpath "$.data.stagedLedgerAccounts[9].balance" == 14120780.833121989
jsonpath "$.data.stagedLedgerAccounts[9].balance_nano" == 14120780833121988
jsonpath "$.data.stagedLedgerAccounts[9].delegate" == "B62qj287L1bwP9XguURbxW5cneTRD8Kde4vx3fbeZCNxNxyMzXdsYLP"
jsonpath "$.data.stagedLedgerAccounts[9].nonce" == 15
jsonpath "$.data.stagedLedgerAccounts[9].publicKey" == "B62qpDWPWYVt7oJTHFME19ic3iFWrZJcEvuLFWHdSSnkrMiQUDZtySU"
jsonpath "$.data.stagedLedgerAccounts[9].username" == null

duration < 10000

# 
# Ledger hash query (block height 359612)
# 

POST {{url}}
```graphql
{
  stagedLedgerAccounts(
    limit: 10
    sortBy: BALANCE_DESC
    query: { ledgerHash: "jxQPAt1PwSKK7kEe6SxgyJDRL7vP4ceCn58qbcGHC2D4Xp2npNU" }
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
jsonpath "$.data.stagedLedgerAccounts" count == 10

# first datum
jsonpath "$.data.stagedLedgerAccounts[0].balance" == 144390363.9093
jsonpath "$.data.stagedLedgerAccounts[0].balance_nano" == 144390363909300000
jsonpath "$.data.stagedLedgerAccounts[0].delegate" == "B62qrQKS9ghd91shs73TCmBJRW9GzvTJK443DPx2YbqcyoLc56g1ny9"
jsonpath "$.data.stagedLedgerAccounts[0].nonce" == 17
jsonpath "$.data.stagedLedgerAccounts[0].publicKey" == "B62qrQKS9ghd91shs73TCmBJRW9GzvTJK443DPx2YbqcyoLc56g1ny9"
jsonpath "$.data.stagedLedgerAccounts[0].username" == null

# last datum
jsonpath "$.data.stagedLedgerAccounts[9].balance" == 14120780.833121989
jsonpath "$.data.stagedLedgerAccounts[9].balance_nano" == 14120780833121988
jsonpath "$.data.stagedLedgerAccounts[9].delegate" == "B62qj287L1bwP9XguURbxW5cneTRD8Kde4vx3fbeZCNxNxyMzXdsYLP"
jsonpath "$.data.stagedLedgerAccounts[9].nonce" == 15
jsonpath "$.data.stagedLedgerAccounts[9].publicKey" == "B62qpDWPWYVt7oJTHFME19ic3iFWrZJcEvuLFWHdSSnkrMiQUDZtySU"
jsonpath "$.data.stagedLedgerAccounts[9].username" == null

duration < 10000

# 
# Public key & state hash query (block height 359612)
# 

POST {{url}}
```graphql
{
  stagedLedgerAccounts(
    query: {
      publicKey: "B62qrgc2UBuyVYZLYU5eS9VFMzSHoKkQGubVm2UXX22q458VSm2Wn9P"
      stateHash: "3NLcUk9u8FgvCip634qaDQFBm26ja8C3pSk2L1SQd9nSE2CEcqpQ"
    }
  ) {
    balance
    balance_nano
    nonce
    publicKey
    username
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.stagedLedgerAccounts" count == 1

# only datum
jsonpath "$.data.stagedLedgerAccounts[0].balance" == 0.0
jsonpath "$.data.stagedLedgerAccounts[0].balance_nano" == 0
jsonpath "$.data.stagedLedgerAccounts[0].nonce" == 1
jsonpath "$.data.stagedLedgerAccounts[0].publicKey" == "B62qrgc2UBuyVYZLYU5eS9VFMzSHoKkQGubVm2UXX22q458VSm2Wn9P"
jsonpath "$.data.stagedLedgerAccounts[0].username" == null

duration < 1000

# 
# Public key & block height query
# 

POST {{url}}
```graphql
{
  stagedLedgerAccounts(
    query: {
      publicKey: "B62qrgc2UBuyVYZLYU5eS9VFMzSHoKkQGubVm2UXX22q458VSm2Wn9P"
      blockchain_length: 359612
    }
  ) {
    balance
    balance_nano
    nonce
    publicKey
    username
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.stagedLedgerAccounts" count == 1

# only datum
jsonpath "$.data.stagedLedgerAccounts[0].balance" == 0.0
jsonpath "$.data.stagedLedgerAccounts[0].balance_nano" == 0
jsonpath "$.data.stagedLedgerAccounts[0].nonce" == 1
jsonpath "$.data.stagedLedgerAccounts[0].publicKey" == "B62qrgc2UBuyVYZLYU5eS9VFMzSHoKkQGubVm2UXX22q458VSm2Wn9P"
jsonpath "$.data.stagedLedgerAccounts[0].username" == null

duration < 1000

# 
# Public key & staged ledger hash query (block height 359612)
# 

POST {{url}}
```graphql
{
  stagedLedgerAccounts(
    query: {
      publicKey: "B62qrgc2UBuyVYZLYU5eS9VFMzSHoKkQGubVm2UXX22q458VSm2Wn9P"
      ledgerHash: "jxQPAt1PwSKK7kEe6SxgyJDRL7vP4ceCn58qbcGHC2D4Xp2npNU"
    }
  ) {
    balance
    balance_nano
    nonce
    publicKey
    username
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.stagedLedgerAccounts" count == 1

# only datum
jsonpath "$.data.stagedLedgerAccounts[0].balance" == 0.0
jsonpath "$.data.stagedLedgerAccounts[0].balance_nano" == 0
jsonpath "$.data.stagedLedgerAccounts[0].nonce" == 1
jsonpath "$.data.stagedLedgerAccounts[0].publicKey" == "B62qrgc2UBuyVYZLYU5eS9VFMzSHoKkQGubVm2UXX22q458VSm2Wn9P"
jsonpath "$.data.stagedLedgerAccounts[0].username" == null

duration < 1000

# 
# Missing public key query (block height 359612)
# 

POST {{url}}
```graphql
{
  stagedLedgerAccounts(
    query: {
      publicKey: "B62qokf5v5s5ABUDiW2wkkPSXSHAwrM7wdgDD4UMSatPRKZgiwvfBnn"
      blockchain_length: 359612
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

duration < 2000
