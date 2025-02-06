#
# Transaction hash query
#

POST {{url}}
```graphql
{
  transaction(
    query: { hash: "CkpaJFpf2q9syHb6dmufSNUii4b5iHvPyakoWgNzKnU7AQ68fzY1P" }
  ) {
    epoch_num_user_commands
    total_num_user_commands
    block {
      stateHash
    }
  }
}
```
HTTP 200
[Asserts]

jsonpath "$.data.transaction.block.stateHash" == "3NLgPYVDSV6y7ZKSQrjowedjZn9RuLwdx311vheWZEy3mBjKDbmj"
jsonpath "$.data.transaction.epoch_num_user_commands" == 303
jsonpath "$.data.transaction.total_num_user_commands" == 303

duration < 100
