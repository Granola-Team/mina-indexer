#
# Explicit token holders query
#

POST {{url}}
```graphql
{
  tokenHolders(
    query: { token: "wfG3GivPMttpt6nQnPuX9eDPnoyA5RJZY23LTc4kkNkCRH2gUd" }
  ) {
    token
    supply
    owner
    symbol
    account {
      publicKey
      delegate
      balance
      nonce
      token
    }
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.tokenHolders" count == 1

# token data
jsonpath "$.data.tokenHolders[0].token" == "wfG3GivPMttpt6nQnPuX9eDPnoyA5RJZY23LTc4kkNkCRH2gUd"
jsonpath "$.data.tokenHolders[0].supply" == 100000000000000
jsonpath "$.data.tokenHolders[0].owner" == "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF"
jsonpath "$.data.tokenHolders[0].symbol" ==  "MINU"

# account data
jsonpath "$.data.tokenHolders[0].account.publicKey" == "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF"
jsonpath "$.data.tokenHolders[0].account.delegate" == "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF"
jsonpath "$.data.tokenHolders[0].account.balance" == 100000000000000
jsonpath "$.data.tokenHolders[0].account.nonce" == 0
jsonpath "$.data.tokenHolders[0].account.token" == "wfG3GivPMttpt6nQnPuX9eDPnoyA5RJZY23LTc4kkNkCRH2gUd"

duration < 100

#
# Explicit holder tokens query
#

POST {{url}}
```graphql
{
  tokenHolders(
    query: { holder: "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF" }
  ) {
    token
    supply
    owner
    symbol
    account {
      publicKey
      delegate
      balance
      nonce
      token
    }
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.tokenHolders" count == 2

# MINA token data
jsonpath "$.data.tokenHolders[*].token" contains "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf"
jsonpath "$.data.tokenHolders[*].supply" contains 1166062074840039233
jsonpath "$.data.tokenHolders[*].owner" contains null
jsonpath "$.data.tokenHolders[*].symbol" contains  "MINA"

# MINA holder data
jsonpath "$.data.tokenHolders[*].account.publicKey" contains "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF"
jsonpath "$.data.tokenHolders[*].account.delegate" contains "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF"
jsonpath "$.data.tokenHolders[*].account.balance" contains 0
jsonpath "$.data.tokenHolders[*].account.nonce" contains 1
jsonpath "$.data.tokenHolders[*].account.token" contains "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf"

# MINU token data
jsonpath "$.data.tokenHolders[*].token" contains "wfG3GivPMttpt6nQnPuX9eDPnoyA5RJZY23LTc4kkNkCRH2gUd"
jsonpath "$.data.tokenHolders[*].supply" contains 100000000000000
jsonpath "$.data.tokenHolders[*].owner" contains "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF"
jsonpath "$.data.tokenHolders[*].symbol" contains  "MINU"

# MINU holder data
jsonpath "$.data.tokenHolders[*].account.publicKey" contains "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF"
jsonpath "$.data.tokenHolders[*].account.delegate" contains "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF"
jsonpath "$.data.tokenHolders[*].account.balance" contains 100000000000000
jsonpath "$.data.tokenHolders[*].account.nonce" contains 0
jsonpath "$.data.tokenHolders[*].account.token" contains "wfG3GivPMttpt6nQnPuX9eDPnoyA5RJZY23LTc4kkNkCRH2gUd"

duration < 100
