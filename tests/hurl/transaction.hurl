POST {{url}}
```graphql
query TransactionQuery {
  transaction(query: {hash: "CkpaJFpf2q9syHb6dmufSNUii4b5iHvPyakoWgNzKnU7AQ68fzY1P"}) {
    block {
      stateHash
    }
  }
}
```
HTTP 200
[Asserts]
jsonpath "$.data.transaction.block.stateHash" == "3NLgPYVDSV6y7ZKSQrjowedjZn9RuLwdx311vheWZEy3mBjKDbmj"
duration < 1000
