#
# Summary endpoint
#

GET {{url}}

HTTP 200
[Asserts]

# total data count
jsonpath "$.totalNumCanonicalSnarks" == 64
jsonpath "$.totalNumInternalCommands" == 399
jsonpath "$.totalNumCanonicalInternalCommands" == 234
jsonpath "$.totalNumCanonicalUserCommands" == 180
jsonpath "$.totalNumAppliedUserCommands" == 287
jsonpath "$.totalNumAppliedCanonicalUserCommands" == 168
jsonpath "$.totalNumFailedUserCommands" == 16
jsonpath "$.totalNumFailedCanonicalUserCommands" == 12
