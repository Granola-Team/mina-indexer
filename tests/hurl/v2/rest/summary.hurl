#
# Summary endpoint
#

GET {{url}}

HTTP 200
[Asserts]

# total data count
jsonpath "$.totalNumCanonicalSnarks" == 0
jsonpath "$.totalNumInternalCommands" == 45
jsonpath "$.totalNumCanonicalInternalCommands" == 23
jsonpath "$.totalNumCanonicalUserCommands" == 97
jsonpath "$.totalNumAppliedUserCommands" == 155
jsonpath "$.totalNumAppliedCanonicalUserCommands" == 97
jsonpath "$.totalNumFailedUserCommands" == 0
jsonpath "$.totalNumFailedCanonicalUserCommands" == 0
