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
duration < 1000
jsonpath "$.data.accounts[0].publicKey" == "B62qj8KB2fk59NkV4VuoTkVXHjw8VJzC3ybKrWo7zuDC9xTiWXPygEe"
jsonpath "$.data.accounts[0].balance" == 719020000000
jsonpath "$.data.accounts[0].nonce" == 1
jsonpath "$.data.accounts[0].delegate" == "B62qj8KB2fk59NkV4VuoTkVXHjw8VJzC3ybKrWo7zuDC9xTiWXPygEe"
