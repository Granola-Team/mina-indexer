#
# Default tokens query
#

POST {{url}}
```graphql
{
  tokens {
    token
    supply
    owner
    symbol
    num_holders
    total_num_txns
    total_num_locked
    total_num_tokens
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.tokens" count == 2

# first datum
jsonpath "$.data.tokens[0].token" == "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf"
jsonpath "$.data.tokens[0].supply" == 1166062074840039233
jsonpath "$.data.tokens[0].owner" == null
jsonpath "$.data.tokens[0].symbol" == "MINA"
jsonpath "$.data.tokens[0].num_holders" == 228181
jsonpath "$.data.tokens[0].total_num_txns" == 97 # we only count the post-hardfork MINA txns here
jsonpath "$.data.tokens[0].total_num_locked" == 0
jsonpath "$.data.tokens[0].total_num_tokens" == 2

# last datum
jsonpath "$.data.tokens[1].token" == "wfG3GivPMttpt6nQnPuX9eDPnoyA5RJZY23LTc4kkNkCRH2gUd"
jsonpath "$.data.tokens[1].supply" == 100000000000000
jsonpath "$.data.tokens[1].owner" == "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF"
jsonpath "$.data.tokens[1].symbol" == "MINU"
jsonpath "$.data.tokens[1].num_holders" == 1
jsonpath "$.data.tokens[1].total_num_txns" == 1
jsonpath "$.data.tokens[1].total_num_locked" == 0
jsonpath "$.data.tokens[1].total_num_tokens" == 2

duration < 100

#
# Token symbol query (MINU)
#

POST {{url}}
```graphql
{
  tokens(query: { symbol: "MINU" }) {
    token
    supply
    owner
    symbol
    num_holders
    total_num_txns
    total_num_locked
    total_num_tokens
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.tokens" count == 1

# first datum
jsonpath "$.data.tokens[0].token" == "wfG3GivPMttpt6nQnPuX9eDPnoyA5RJZY23LTc4kkNkCRH2gUd"
jsonpath "$.data.tokens[0].supply" == 100000000000000
jsonpath "$.data.tokens[0].owner" == "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF"
jsonpath "$.data.tokens[0].symbol" == "MINU"
jsonpath "$.data.tokens[0].num_holders" == 1
jsonpath "$.data.tokens[0].total_num_txns" == 1
jsonpath "$.data.tokens[0].total_num_locked" == 0
jsonpath "$.data.tokens[0].total_num_tokens" == 2

duration < 100

#
# Token symbol query (MINA)
#

POST {{url}}
```graphql
{
  tokens(query: {symbol: "MINA" }) {
    token
    supply
    owner
    symbol
    num_holders
    total_num_txns
    total_num_locked
    total_num_tokens
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.tokens" count == 1

# first datum
jsonpath "$.data.tokens[0].token" == "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf"
jsonpath "$.data.tokens[0].supply" == 1166062074840039233
jsonpath "$.data.tokens[0].owner" == null
jsonpath "$.data.tokens[0].symbol" == "MINA"
jsonpath "$.data.tokens[0].num_holders" == 228181
jsonpath "$.data.tokens[0].total_num_txns" == 97 # we only count the post-hardfork MINA txns here
jsonpath "$.data.tokens[0].total_num_locked" == 0
jsonpath "$.data.tokens[0].total_num_tokens" == 2
