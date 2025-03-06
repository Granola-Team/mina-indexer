#
# Explicit token holders query
#

POST {{url}}
```graphql
{
  tokenHolders(
    query: {
      token: "wfG3GivPMttpt6nQnPuX9eDPnoyA5RJZY23LTc4kkNkCRH2gUd"
    }) {
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
jsonpath "$.data.tokenHolders[0].symbol" ==  ""

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
    query: {
      holder: "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF"
    }) {
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
jsonpath "$.data.tokenHolders[0].symbol" ==  ""

# account data
jsonpath "$.data.tokenHolders[0].account.publicKey" == "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF"
jsonpath "$.data.tokenHolders[0].account.delegate" == "B62qkPg6P2We1SZhCq84ZvDKknrWy8P3Moi99Baz8KFpYsMoFJKHHqF"
jsonpath "$.data.tokenHolders[0].account.balance" == 100000000000000
jsonpath "$.data.tokenHolders[0].account.nonce" == 0
jsonpath "$.data.tokenHolders[0].account.token" == "wfG3GivPMttpt6nQnPuX9eDPnoyA5RJZY23LTc4kkNkCRH2gUd"

duration < 100
