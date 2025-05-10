#
# Summary endpoint
#

GET {{url}}

HTTP 200
[Asserts]

# chain
jsonpath "$.chainId" == "a7351abc7ddf2ea92d1b38cc8e636c271c1dfd2c081c637f62ebc2af34eb7cc1"
jsonpath "$.genesisStateHash" == "3NK4BpDSekaqsG6tx8Nse2zJchRft2JpnbvMiog55WCr5xJZaKeP"

jsonpath "$.blockchainLength" == 359617

# accounts
jsonpath "$.totalNumAccounts" == 228180
jsonpath "$.totalNumMinaAccounts" == 228179
jsonpath "$.totalNumZkappAccounts" == 6
jsonpath "$.totalNumMinaZkappAccounts" == 0

# blocks
jsonpath "$.epochNumBlocks" == 24
jsonpath "$.totalNumBlocks" == 38
jsonpath "$.numUniqueBlockProducers" == 26

# canonical blocks
jsonpath "$.epochNumCanonicalBlocks" == 13

# canonical snarks
jsonpath "$.totalNumCanonicalSnarks" == 0

# snarks
jsonpath "$.epochNumSnarks" == 0
jsonpath "$.totalNumSnarks" == 0

# canonical snarks
jsonpath "$.totalNumCanonicalSnarks" == 0

# internal commands
jsonpath "$.epochNumInternalCommands" == 45
jsonpath "$.totalNumInternalCommands" == 45

# canonical internal commands
jsonpath "$.totalNumCanonicalInternalCommands" == 23

# user commands
jsonpath "$.epochNumUserCommands" == 155
jsonpath "$.totalNumUserCommands" == 155

# canonical user commands
jsonpath "$.totalNumCanonicalUserCommands" == 97

# applied user commands
jsonpath "$.totalNumAppliedUserCommands" == 155

# applied canonical user commands
jsonpath "$.totalNumAppliedCanonicalUserCommands" == 97

# failed user commands
jsonpath "$.totalNumFailedUserCommands" == 0

# failed canonical user commands
jsonpath "$.totalNumFailedCanonicalUserCommands" == 0

# zkapp commands
jsonpath "$.epochNumZkappCommands" == 80
jsonpath "$.totalNumZkappCommands" == 80

# canonical zkapp commands
jsonpath "$.totalNumCanonicalZkappCommands" == 74

# applied zkapp commands
jsonpath "$.totalNumAppliedZkappCommands" == 80

# applied canonical zkapp commands
jsonpath "$.totalNumAppliedCanonicalZkappCommands" == 74

# failed zkapp commands
jsonpath "$.totalNumFailedZkappCommands" == 0

# failed canonical zkapp commands
jsonpath "$.totalNumFailedCanonicalZkappCommands" == 0
