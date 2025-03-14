#
# Genesis state hash bounded query (absent)
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
    ledger_hash
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.stakes" count == 1

# only datum
jsonpath "$.data.stakes[0].epoch" == 0
jsonpath "$.data.stakes[0].ledger_hash" == "jxsAidvKvEQJMC7Z2wkLrFGzCqUxpFMRhAj4K5o49eiFLhKSyXL"

duration < 500
