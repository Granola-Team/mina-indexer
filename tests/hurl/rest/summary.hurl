#
# Summary endpoint
#

GET {{url}}

HTTP 200
[Asserts]

# total data count
jsonpath "$.totalNumCanonicalSnarks" == 64
jsonpath "$.totalNumCanonicalUserCommands" == 180
jsonpath "$.totalNumAppliedUserCommands" == 287
jsonpath "$.totalNumAppliedCanonicalUserCommands" == 168
jsonpath "$.totalNumFailedUserCommands" == 16
jsonpath "$.totalNumFailedCanonicalUserCommands" == 12
