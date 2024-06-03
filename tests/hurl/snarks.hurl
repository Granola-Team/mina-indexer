#
# SNARKs canonical block height descending query
#

POST {{url}}
```graphql
query Snarks($limit: Int = 10, $sort_by: SnarkSortByInput!, $query: SnarkQueryInput!) {
  snarks(limit: $limit, sortBy: $sort_by, query: $query ) {
    canonical
    blockHeight
    prover
    fee
    dateTime
    block {
      stateHash
    }
  }
}

variables {
  "limit": 10,
  "sort_by": "BLOCKHEIGHT_DESC",
  "query": {
    "canonical": true
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.snarks" count == 10

# first datum
jsonpath "$.data.snarks[0].dateTime" == "2021-03-17T08:06:00.000Z"
jsonpath "$.data.snarks[0].canonical" == true
jsonpath "$.data.snarks[0].blockHeight" == 111
jsonpath "$.data.snarks[0].prover" == "B62qrCz3ehCqi8Pn8y3vWC9zYEB9RKsidauv15DeZxhzkxL3bKeba5h"
jsonpath "$.data.snarks[0].fee" == 0
jsonpath "$.data.snarks[0].block.stateHash" == "3NL33j16AWm3Jhjj1Ud25E54hu7HpUq4WBQcAiijEKMfXqwFJwzK"

# last datum
jsonpath "$.data.snarks[9].dateTime" == "2021-03-17T08:06:00.000Z"
jsonpath "$.data.snarks[9].canonical" == true
jsonpath "$.data.snarks[9].blockHeight" == 111
jsonpath "$.data.snarks[9].prover" == "B62qrCz3ehCqi8Pn8y3vWC9zYEB9RKsidauv15DeZxhzkxL3bKeba5h"
jsonpath "$.data.snarks[9].fee" == 0
jsonpath "$.data.snarks[9].block.stateHash" == "3NL33j16AWm3Jhjj1Ud25E54hu7HpUq4WBQcAiijEKMfXqwFJwzK"

duration < 500

#
# SNARKs canonical block height query
#

POST {{url}}
```graphql
{
  snarks(query: {canonical: true, blockHeight: 111}, limit: 1000) {
    blockHeight
    canonical
    dateTime
    fee
    prover
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.snarks" count == 64

# first datum
jsonpath "$.data.snarks[0].dateTime" == "2021-03-17T08:06:00.000Z"
jsonpath "$.data.snarks[0].canonical" == true
jsonpath "$.data.snarks[0].blockHeight" == 111
jsonpath "$.data.snarks[0].prover" == "B62qrCz3ehCqi8Pn8y3vWC9zYEB9RKsidauv15DeZxhzkxL3bKeba5h"
jsonpath "$.data.snarks[0].fee" == 0

# last datum
jsonpath "$.data.snarks[63].dateTime" == "2021-03-17T08:06:00.000Z"
jsonpath "$.data.snarks[63].canonical" == true
jsonpath "$.data.snarks[63].blockHeight" == 111
jsonpath "$.data.snarks[63].prover" == "B62qrCz3ehCqi8Pn8y3vWC9zYEB9RKsidauv15DeZxhzkxL3bKeba5h"
jsonpath "$.data.snarks[63].fee" == 0

duration < 1000

#
# SNARKs canonical state hash query
#

POST {{url}}
```graphql
{
  snarks(query: {canonical: true, block: {stateHash: "3NL33j16AWm3Jhjj1Ud25E54hu7HpUq4WBQcAiijEKMfXqwFJwzK"}}) {
		blockHeight
    block {
      stateHash
    }
    epoch_num_snarks
    total_num_snarks
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.snarks" count == 64

# first datum
jsonpath "$.data.snarks[0].blockHeight" == 111
jsonpath "$.data.snarks[0].block.stateHash" == "3NL33j16AWm3Jhjj1Ud25E54hu7HpUq4WBQcAiijEKMfXqwFJwzK"
jsonpath "$.data.snarks[0].epoch_num_snarks" == 64
jsonpath "$.data.snarks[0].total_num_snarks" == 64

# last datum
jsonpath "$.data.snarks[63].blockHeight" == 111
jsonpath "$.data.snarks[63].block.stateHash" == "3NL33j16AWm3Jhjj1Ud25E54hu7HpUq4WBQcAiijEKMfXqwFJwzK"

duration < 500

#
# SNARKs by prover query
#

POST {{url}}
```graphql
{
  snarks(query: {prover: "B62qrCz3ehCqi8Pn8y3vWC9zYEB9RKsidauv15DeZxhzkxL3bKeba5h"}, limit: 100) {
    prover
    canonical
    blockHeight
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.snarks" count == 64

# first datum
jsonpath "$.data.snarks[0].prover" == "B62qrCz3ehCqi8Pn8y3vWC9zYEB9RKsidauv15DeZxhzkxL3bKeba5h"
jsonpath "$.data.snarks[0].canonical" == true
jsonpath "$.data.snarks[0].blockHeight" == 111

# last datum
jsonpath "$.data.snarks[63].prover" == "B62qrCz3ehCqi8Pn8y3vWC9zYEB9RKsidauv15DeZxhzkxL3bKeba5h"
jsonpath "$.data.snarks[63].canonical" == true
jsonpath "$.data.snarks[63].blockHeight" == 111

duration < 500

#
# SNARKs canonical block height lte
#

POST {{url}}
```graphql
{
  snarks(query: {canonical: true, blockHeight_lte: 120}, limit: 1000) {
    blockHeight
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.snarks" count == 64

duration < 500

# TODO global slot query
# TODO block height/global slot/date time bounded query
