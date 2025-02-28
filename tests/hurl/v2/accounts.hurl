#
# Explicit zkapp accounts public key query
#

POST {{url}}
```graphql
{
  accounts(
    query: {
      publicKey: "B62qrgc2UBuyVYZLYU5eS9VFMzSHoKkQGubVm2UXX22q458VSm2Wn9P",
      zkapp: true
    }
  ) {
    publicKey
    balance
    username
    nonce
    delegate
    timeLocked
    is_genesis_account
    token
    zkapp {
      appState,
      zkappUri
    }
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.accounts" count == 1

# only datum
jsonpath "$.data.accounts[0].publicKey" == "B62qrgc2UBuyVYZLYU5eS9VFMzSHoKkQGubVm2UXX22q458VSm2Wn9P"
jsonpath "$.data.accounts[0].balance" == 0
jsonpath "$.data.accounts[0].nonce" == 1
jsonpath "$.data.accounts[0].delegate" == "B62qrgc2UBuyVYZLYU5eS9VFMzSHoKkQGubVm2UXX22q458VSm2Wn9P"
jsonpath "$.data.accounts[0].timeLocked" == false
jsonpath "$.data.accounts[0].is_genesis_account" == false
jsonpath "$.data.accounts[0].token" == "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf"

# zkapp data
jsonpath "$.data.accounts[0].zkapp.zkappUri" == ""
jsonpath "$.data.accounts[0].zkapp.appState[0]" == "0x0000000000000000000000000000000000000000000000000000000000000001"
jsonpath "$.data.accounts[0].zkapp.appState[1]" == "0x0000000000000000000000000000000000000000000000000000000000000000"
jsonpath "$.data.accounts[0].zkapp.appState[2]" == "0x0000000000000000000000000000000000000000000000000000000000000000"
jsonpath "$.data.accounts[0].zkapp.appState[3]" == "0x0000000000000000000000000000000000000000000000000000000000000000"
jsonpath "$.data.accounts[0].zkapp.appState[4]" == "0x0000000000000000000000000000000000000000000000000000000000000000"
jsonpath "$.data.accounts[0].zkapp.appState[5]" == "0x0000000000000000000000000000000000000000000000000000000000000000"
jsonpath "$.data.accounts[0].zkapp.appState[6]" == "0x0000000000000000000000000000000000000000000000000000000000000000"
jsonpath "$.data.accounts[0].zkapp.appState[7]" == "0x0000000000000000000000000000000000000000000000000000000000000000"

duration < 100

#
# Implicit zkapp accounts public key query
#

POST {{url}}
```graphql
{
  accounts(query: { publicKey: "B62qrgc2UBuyVYZLYU5eS9VFMzSHoKkQGubVm2UXX22q458VSm2Wn9P" }) {
    publicKey
    balance
    token
    username
    nonce
    delegate
    timeLocked
    is_genesis_account
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.accounts" count == 1

# only datum
jsonpath "$.data.accounts[0].publicKey" == "B62qrgc2UBuyVYZLYU5eS9VFMzSHoKkQGubVm2UXX22q458VSm2Wn9P"
jsonpath "$.data.accounts[0].balance" == 0
jsonpath "$.data.accounts[0].nonce" == 1
jsonpath "$.data.accounts[0].delegate" == "B62qrgc2UBuyVYZLYU5eS9VFMzSHoKkQGubVm2UXX22q458VSm2Wn9P"
jsonpath "$.data.accounts[0].timeLocked" == false
jsonpath "$.data.accounts[0].is_genesis_account" == false
jsonpath "$.data.accounts[0].token" == "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf"

duration < 100

#
# Balance-sorted zkapp accounts query
#

POST {{url}}
```graphql
{
  accounts(
    query: {
      zkapp: true
    }
    sortBy: BALANCE_DESC
  ) {
    publicKey
    balance
    token
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.accounts" count == 2

# data
jsonpath "$.data.accounts[0].publicKey" == "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF"
jsonpath "$.data.accounts[0].balance" == 2000000000
jsonpath "$.data.accounts[0].token" == "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf"

jsonpath "$.data.accounts[1].publicKey" == "B62qrgc2UBuyVYZLYU5eS9VFMzSHoKkQGubVm2UXX22q458VSm2Wn9P"
jsonpath "$.data.accounts[1].balance" == 0
jsonpath "$.data.accounts[1].token" == "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf"

duration < 100

#
# Balance-sorted zkapp accounts query
#

POST {{url}}
```graphql
{
  accounts(
    query: {
      zkapp: true
    }
    sortBy: BALANCE_ASC
  ) {
    publicKey
    balance
    token
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.accounts" count == 2

# data
jsonpath "$.data.accounts[0].publicKey" == "B62qrgc2UBuyVYZLYU5eS9VFMzSHoKkQGubVm2UXX22q458VSm2Wn9P"
jsonpath "$.data.accounts[0].balance" == 0
jsonpath "$.data.accounts[0].token" == "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf"

jsonpath "$.data.accounts[1].publicKey" == "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF"
jsonpath "$.data.accounts[1].balance" == 2000000000
jsonpath "$.data.accounts[1].token" == "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf"

#
# Balance-sorted all accounts query
#

POST {{url}}
```graphql
{
  accounts(
    sortBy: BALANCE_DESC,
    limit: 5
  ) {
    publicKey
    balance
    token
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.accounts" count == 5

# data
jsonpath "$.data.accounts[0].publicKey" == "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF"
jsonpath "$.data.accounts[0].balance" == 99999000000000
jsonpath "$.data.accounts[0].token" == "wfG3GivPMttpt6nQnPuX9eDPnoyA5RJZY23LTc4kkNkCRH2gUd"

jsonpath "$.data.accounts[1].publicKey" == "B62qrQKS9ghd91shs73TCmBJRW9GzvTJK443DPx2YbqcyoLc56g1ny9"
jsonpath "$.data.accounts[1].balance" == 144390363909300000
jsonpath "$.data.accounts[1].token" == "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf"

jsonpath "$.data.accounts[2].publicKey" == "B62qjsFTBw4TVwRRxNVrmwJfQqXfmMC4DVa2moCe9f8ErvBYd6f7npr"
jsonpath "$.data.accounts[2].balance" == 105449105983889012
jsonpath "$.data.accounts[2].token" == "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf"

jsonpath "$.data.accounts[3].publicKey" == "B62qp3LaAUKQ76DdFYaQ7bj46HDTgpCaFpwhDqbjNJUC79Rf6x8CxV3"
jsonpath "$.data.accounts[3].balance" == 36842291710745543
jsonpath "$.data.accounts[3].token" == "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf"

jsonpath "$.data.accounts[4].publicKey" == "B62qpWaQoQoPL5AGta7Hz2DgJ9CJonpunjzCGTdw8KiCCD1hX8fNHuR"
jsonpath "$.data.accounts[4].balance" == 31748373259028374
jsonpath "$.data.accounts[4].token" == "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf"

duration < 100

#
# Balance-sorted all accounts query
#

POST {{url}}
```graphql
{
  accounts(
    query: {
      token: "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf"
    },
    sortBy: BALANCE_DESC,
    limit: 5
  ) {
    publicKey
    balance
    token
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.accounts" count == 5

# data
jsonpath "$.data.accounts[0].publicKey" == "B62qrQKS9ghd91shs73TCmBJRW9GzvTJK443DPx2YbqcyoLc56g1ny9"
jsonpath "$.data.accounts[0].balance" == 144390363909300000
jsonpath "$.data.accounts[0].token" == "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf"

jsonpath "$.data.accounts[1].publicKey" == "B62qjsFTBw4TVwRRxNVrmwJfQqXfmMC4DVa2moCe9f8ErvBYd6f7npr"
jsonpath "$.data.accounts[1].balance" == 105449105983889012
jsonpath "$.data.accounts[1].token" == "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf"

jsonpath "$.data.accounts[2].publicKey" == "B62qp3LaAUKQ76DdFYaQ7bj46HDTgpCaFpwhDqbjNJUC79Rf6x8CxV3"
jsonpath "$.data.accounts[2].balance" == 36842291710745543
jsonpath "$.data.accounts[2].token" == "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf"

jsonpath "$.data.accounts[3].publicKey" == "B62qpWaQoQoPL5AGta7Hz2DgJ9CJonpunjzCGTdw8KiCCD1hX8fNHuR"
jsonpath "$.data.accounts[3].balance" == 31748373259028374
jsonpath "$.data.accounts[3].token" == "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf"

jsonpath "$.data.accounts[4].publicKey" == "B62qjVWZ51Uetk83aFbtnVFsxiPzSRxJLEfRXbwGj4G7YzxfgBzv9gq"
jsonpath "$.data.accounts[4].balance" == 20000000400000000
jsonpath "$.data.accounts[4].token" == "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf"

duration < 100

#
# Balance-sorted zkapp
#

POST {{url}}
```graphql
{
  accounts(
    query: {
      token: "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf",
      zkapp: true
    }
    sortBy: BALANCE_DESC
  ) {
    publicKey
    balance
    token
    nonce
    delegate
    timeLocked
    is_genesis_account
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.accounts" count == 2

# only datum
jsonpath "$.data.accounts[0].publicKey" == "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF"
jsonpath "$.data.accounts[0].balance" == 2000000000
jsonpath "$.data.accounts[0].nonce" == 3
jsonpath "$.data.accounts[0].delegate" == "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF"
jsonpath "$.data.accounts[0].timeLocked" == false
jsonpath "$.data.accounts[0].is_genesis_account" == false
jsonpath "$.data.accounts[0].token" == "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf"

jsonpath "$.data.accounts[1].publicKey" == "B62qrgc2UBuyVYZLYU5eS9VFMzSHoKkQGubVm2UXX22q458VSm2Wn9P"
jsonpath "$.data.accounts[1].balance" == 0
jsonpath "$.data.accounts[1].nonce" == 1
jsonpath "$.data.accounts[1].delegate" == "B62qrgc2UBuyVYZLYU5eS9VFMzSHoKkQGubVm2UXX22q458VSm2Wn9P"
jsonpath "$.data.accounts[1].timeLocked" == false
jsonpath "$.data.accounts[1].is_genesis_account" == false
jsonpath "$.data.accounts[1].token" == "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf"

duration < 100
