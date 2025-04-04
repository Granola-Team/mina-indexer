#
# Summary endpoint
#

GET {{url}}

HTTP 200
[Asserts]

# only datum
jsonpath "$.chainId" == "5f704cc0c82e0ed70e873f0893d7e06f148524e3f0bdae2afb02e7819a0c24d1"
jsonpath "$.genesisStateHash" == "3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ"

jsonpath "$.totalNumCanonicalSnarks" == 64
jsonpath "$.totalNumInternalCommands" == 399
jsonpath "$.totalNumCanonicalInternalCommands" == 234
jsonpath "$.totalNumCanonicalUserCommands" == 180
jsonpath "$.totalNumAppliedUserCommands" == 287
jsonpath "$.totalNumAppliedCanonicalUserCommands" == 168
jsonpath "$.totalNumFailedUserCommands" == 16
jsonpath "$.totalNumFailedCanonicalUserCommands" == 12
jsonpath "$.numUniqueBlockProducers" == 86
