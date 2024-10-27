#
# Top SNARKers query
#

POST {{url}}
```graphql
{
  topSnarkers(limit: 100, query: { epoch: 0 }) {
    public_key
    username
    total_fees
    min_fee
    max_fee
    snarks_sold
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.topSnarkers" count == 1

# check data
jsonpath "$.data.topSnarkers[0].public_key" == "B62qrCz3ehCqi8Pn8y3vWC9zYEB9RKsidauv15DeZxhzkxL3bKeba5h"
jsonpath "$.data.topSnarkers[0].username" == "Unknown"
jsonpath "$.data.topSnarkers[0].total_fees" == 0
jsonpath "$.data.topSnarkers[0].min_fee" == 0
jsonpath "$.data.topSnarkers[0].max_fee" == 0
jsonpath "$.data.topSnarkers[0].snarks_sold" == 64

duration < 100
