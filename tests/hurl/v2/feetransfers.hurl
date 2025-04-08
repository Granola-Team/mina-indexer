#
# Fee transfers genesis block by state hash
#

POST {{url}}
```graphql
{
  feetransfers(query: {
    blockStateHash: {
      stateHash: "3NK4BpDSekaqsG6tx8Nse2zJchRft2JpnbvMiog55WCr5xJZaKeP"
    }
  }) {
    blockHeight
    canonical
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.feetransfers" count == 0

duration < 100

#
# Fee transfers v2 genesis block by block height & canonicity
#

POST {{url}}
```graphql
{
  feetransfers(query: {
    blockHeight: 359605
    canonical: true
  }) {
    blockStateHash {
      stateHash
    }
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.feetransfers" count == 0

duration < 100

#
# Fee transfers per pk
#

POST {{url}}
```graphql
{
  feetransfers(query: {
    recipient: "B62qiy32p8kAKnny8ZFwoMhYpBppM1DWVCqAPBYNcXnsAHhnfAAuXgg"
  }) {
    blockStateHash {
      stateHash
    }
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.feetransfers" count == 0

duration < 100