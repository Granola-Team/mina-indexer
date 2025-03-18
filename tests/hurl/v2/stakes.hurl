#
# Post-hardfork stakes query (epoch 0 present)
#

POST {{url}}
```graphql
{
  stakes(
    limit: 1
    query: {
      epoch: 0
      genesisStateHash: "3NK4BpDSekaqsG6tx8Nse2zJchRft2JpnbvMiog55WCr5xJZaKeP"
    }
  ) {
    epoch
    ledgerHash
    genesisStateHash
    epoch_num_accounts
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.stakes" count == 1

# only datum
jsonpath "$.data.stakes[0].epoch" == 0
jsonpath "$.data.stakes[0].ledgerHash" == "jxsAidvKvEQJMC7Z2wkLrFGzCqUxpFMRhAj4K5o49eiFLhKSyXL"
jsonpath "$.data.stakes[0].genesisStateHash" == "3NK4BpDSekaqsG6tx8Nse2zJchRft2JpnbvMiog55WCr5xJZaKeP"
jsonpath "$.data.stakes[0].epoch_num_accounts" == 226659

duration < 500

#
# Post-hardfork stakes query (epoch 1 absent)
#

POST {{url}}
```graphql
{
  stakes(
    limit: 1
    query: {
      epoch: 1
      genesisStateHash: "3NK4BpDSekaqsG6tx8Nse2zJchRft2JpnbvMiog55WCr5xJZaKeP"
    }
  ) {
    epoch
  }
}
```
HTTP 200
[Asserts]

# no data
jsonpath "$.data.stakes" count == 0

duration < 500

#
# Pre-hardfork stakes query (epoch 0 present)
#

POST {{url}}
```graphql
{
  stakes(
    limit: 1
    query: {
      epoch: 0
      genesisStateHash: "3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ"
    }
  ) {
    epoch
    ledgerHash
    genesisStateHash
    epoch_num_accounts
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.stakes" count == 1

# only datum
jsonpath "$.data.stakes[0].epoch" == 0
jsonpath "$.data.stakes[0].ledgerHash" == "jx7buQVWFLsXTtzRgSxbYcT8EYLS8KCZbLrfDcJxMtyy4thw2Ee"
jsonpath "$.data.stakes[0].genesisStateHash" == "3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ"
jsonpath "$.data.stakes[0].epoch_num_accounts" == 1676

duration < 500

#
# Pre-hardfork stakes query (epoch 2 absent)
#

POST {{url}}
```graphql
{
  stakes(
    limit: 1
    query: {
      epoch: 2
      genesisStateHash: "3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ"
    }
  ) {
    epoch
  }
}
```
HTTP 200
[Asserts]

# no data
jsonpath "$.data.stakes" count == 0

duration < 500
