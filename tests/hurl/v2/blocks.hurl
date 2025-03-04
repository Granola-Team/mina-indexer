#
# Blocks canonical height descending query
#

POST {{url}}
```graphql
{
  blocks(limit: 13, sortBy: BLOCKHEIGHT_DESC, query: { canonical: true }) {
    stateHash
    txFees
    blockHeight
    canonical
    creator
    dateTime
    receivedTime
    snarkFees
    block_num_user_commands
    block_num_zkapp_commands
  }
}
```
HTTP 200
[Asserts]

# total query results
jsonpath "$.data.blocks" count == 13

# first datum
jsonpath "$.data.blocks[0].stateHash" == "3NLXcoYjz1bqK3QzKZYmQxa7kZ7CyLqLisGLgHH6vikHokW3xYz6"
jsonpath "$.data.blocks[0].blockHeight" == 359617
jsonpath "$.data.blocks[0].txFees" == "800000000"
jsonpath "$.data.blocks[0].canonical" == true
jsonpath "$.data.blocks[0].creator" == "B62qq3TQ8AP7MFYPVtMx5tZGF3kWLJukfwG1A1RGvaBW1jfTPTkDBW6"
jsonpath "$.data.blocks[0].dateTime" == "2024-06-05T00:54:00.000Z"
jsonpath "$.data.blocks[0].receivedTime" == "2024-06-05T00:54:08.882Z"
jsonpath "$.data.blocks[0].snarkFees" == "0"
jsonpath "$.data.blocks[0].block_num_user_commands" == 8
jsonpath "$.data.blocks[0].block_num_zkapp_commands" == 7

# last datum
jsonpath "$.data.blocks[12].stateHash" == "3NK4BpDSekaqsG6tx8Nse2zJchRft2JpnbvMiog55WCr5xJZaKeP"
jsonpath "$.data.blocks[12].blockHeight" == 359605
jsonpath "$.data.blocks[12].txFees" == "0"
jsonpath "$.data.blocks[12].canonical" == true
jsonpath "$.data.blocks[12].creator" == "B62qiy32p8kAKnny8ZFwoMhYpBppM1DWVCqAPBYNcXnsAHhnfAAuXgg"
jsonpath "$.data.blocks[12].dateTime" == "2024-06-05T00:03:00.000Z"
jsonpath "$.data.blocks[12].receivedTime" == "2024-06-05T00:00:00.000Z"
jsonpath "$.data.blocks[12].snarkFees" == "0"
jsonpath "$.data.blocks[12].block_num_user_commands" == 0
jsonpath "$.data.blocks[12].block_num_zkapp_commands" == 0

duration < 10000
