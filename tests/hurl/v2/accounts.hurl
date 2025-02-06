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

duration < 100
