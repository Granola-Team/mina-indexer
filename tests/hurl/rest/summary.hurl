#
# Summary endpoint
#

GET {{url}}

HTTP 200
[Asserts]

# total data count
jsonpath "$.totalNumCanonicalSnarks" == 64
