#
# Internal commands genesis block by state hash
#

POST {{url}}
```graphql
{
  internalCommands(
    query: {
      blockStateHash: {
        stateHash: "3NK4BpDSekaqsG6tx8Nse2zJchRft2JpnbvMiog55WCr5xJZaKeP"
      }
    }
  ) {
    blockHeight
    canonical
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.internalCommands" count == 0

duration < 100

#
# Internal commands v2 genesis block by block height & canonicity
#

POST {{url}}
```graphql
{
  internalCommands(query: { blockHeight: 359605, canonical: true }) {
    blockStateHash {
      stateHash
    }
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.internalCommands" count == 0

duration < 100

#
# Internal commands per pk
#

POST {{url}}
```graphql
{
  internalCommands(
    query: {
      recipient: "B62qiy32p8kAKnny8ZFwoMhYpBppM1DWVCqAPBYNcXnsAHhnfAAuXgg"
    }
  ) {
    blockStateHash {
      stateHash
    }
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.internalCommands" count == 0

duration < 100