#
# Top SNARKers query
#

POST {{url}}
```graphql
{
  topSnarkers(query: { epoch: 0 }) {
    public_key
    username
    total_fees
    epoch_fees
    min_fee
    epoch_min_fee
    max_fee
    epoch_max_fee
    snarks_sold
    epoch_snarks_sold
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.topSnarkers" count == 1

# check data
jsonpath "$.data.topSnarkers[0].public_key" == "B62qrCz3ehCqi8Pn8y3vWC9zYEB9RKsidauv15DeZxhzkxL3bKeba5h"
jsonpath "$.data.topSnarkers[0].username" == null
jsonpath "$.data.topSnarkers[0].total_fees" == 0
jsonpath "$.data.topSnarkers[0].epoch_fees" == 0
jsonpath "$.data.topSnarkers[0].min_fee" == 0
jsonpath "$.data.topSnarkers[0].epoch_min_fee" == 0
jsonpath "$.data.topSnarkers[0].max_fee" == 0
jsonpath "$.data.topSnarkers[0].epoch_max_fee" == 0
jsonpath "$.data.topSnarkers[0].snarks_sold" == 64
jsonpath "$.data.topSnarkers[0].epoch_snarks_sold" == 64

duration < 100
