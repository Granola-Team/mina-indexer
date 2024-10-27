#
# SNARKs canonical block height descending query
#

POST {{url}}
```graphql
{
  snarks(limit: 10, sortBy: BLOCKHEIGHT_DESC, query: { canonical: true }) {
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
  snarks(limit: 1000, query: { canonical: true, blockHeight: 111 }) {
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

duration < 500

#
# SNARKs canonical state hash query
#

POST {{url}}
```graphql
{
  snarks(
    query: {
      canonical: true
      block: {
        stateHash: "3NL33j16AWm3Jhjj1Ud25E54hu7HpUq4WBQcAiijEKMfXqwFJwzK"
      }
    }
  ) {
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
  snarks(
    limit: 100
    query: {
      prover: "B62qrCz3ehCqi8Pn8y3vWC9zYEB9RKsidauv15DeZxhzkxL3bKeba5h"
      canonical: true
    }
  ) {
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
  snarks(limit: 1000, query: { canonical: true, blockHeight_lte: 120 }) {
    blockHeight
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.snarks" count == 64

jsonpath "$.data.snarks[0].blockHeight" == 111
jsonpath "$.data.snarks[63].blockHeight" == 111

duration < 500

#
# SNARKs filter by prover and sort by blockheight
#

POST {{url}}
```graphql
{
  snarks(
    limit: 100
    sortBy: BLOCKHEIGHT_ASC
    query: {
      prover: "B62qrCz3ehCqi8Pn8y3vWC9zYEB9RKsidauv15DeZxhzkxL3bKeba5h"
      canonical: true
      blockHeight_lte: 120
    }
  ) {
    canonical
    blockHeight
    block {
      stateHash
    }
    fee
    dateTime
    prover
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.snarks" count == 64

jsonpath "$.data.snarks[0].blockHeight" == 111
jsonpath "$.data.snarks[0].prover" == "B62qrCz3ehCqi8Pn8y3vWC9zYEB9RKsidauv15DeZxhzkxL3bKeba5h"
jsonpath "$.data.snarks[63].blockHeight" == 111
jsonpath "$.data.snarks[63].prover" == "B62qrCz3ehCqi8Pn8y3vWC9zYEB9RKsidauv15DeZxhzkxL3bKeba5h"

duration < 500

#
# SNARKs filtered by prover and sorted by block height
#

POST {{url}}
```graphql
{
  snarks(
    limit: 100
    sortBy: BLOCKHEIGHT_DESC
    query: {
      prover: "B62qrCz3ehCqi8Pn8y3vWC9zYEB9RKsidauv15DeZxhzkxL3bKeba5h"
      canonical: true
      blockHeight_lte: 111
    }
  ) {
    blockHeight
    prover
    canonical
  }
}
```
HTTP 200
[Asserts]

# data count
jsonpath "$.data.snarks" count == 64

# first datum
jsonpath "$.data.snarks[0].blockHeight" == 111
jsonpath "$.data.snarks[0].canonical" == true
jsonpath "$.data.snarks[0].prover" == "B62qrCz3ehCqi8Pn8y3vWC9zYEB9RKsidauv15DeZxhzkxL3bKeba5h"

# last datum
jsonpath "$.data.snarks[63].blockHeight" == 111
jsonpath "$.data.snarks[63].canonical" == true
jsonpath "$.data.snarks[63].prover" == "B62qrCz3ehCqi8Pn8y3vWC9zYEB9RKsidauv15DeZxhzkxL3bKeba5h"

duration < 500
