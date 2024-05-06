# 
# State hash query
# 

POST {{url}}
```graphql
query StagedLedgerAccounts($limit: Int, $sort_by: StagedLedgerSortByInput!, $query: StagedLedgerQueryInput!) {
  stagedLedgerAccounts(limit: $limit, sortBy: $sort_by, query: $query) {
    balance
    balance_nanomina
    delegate
    nonce
    public_key
    username
  }
}

variables {
  "limit": 100,
  "sort_by": "BALANCE_DESC",
  "query": {
    "stateHash": "3NKeMoncuHab5ScarV5ViyF16cJPT4taWNSaTLS64Dp67wuXigPZ"
  }
}
```
HTTP 200
[Asserts]

# Total accounts returned
jsonpath "$.data.stagedLedgerAccounts" count == 100

jsonpath "$.data.stagedLedgerAccounts[0].balance" == 75000000.0
jsonpath "$.data.stagedLedgerAccounts[0].balance_nanomina" == 75000000
jsonpath "$.data.stagedLedgerAccounts[0].delegate" == "B62qqhURJQo3CvWC3WFo9LhUhtcaJWLBcJsaA3DXaU2GH5KgXujZiwB"
jsonpath "$.data.stagedLedgerAccounts[0].nonce" == 0
jsonpath "$.data.stagedLedgerAccounts[0].public_key" == "B62qpbZkvpHZ1a5nsTbANuRtrdw4YraTyA4nvJDm6HpP1YMC9QStxX3"
jsonpath "$.data.stagedLedgerAccounts[0].username" == null

jsonpath "$.data.stagedLedgerAccounts[99].balance" == 1693980.63775165
jsonpath "$.data.stagedLedgerAccounts[99].balance_nanomina" == 1693980
jsonpath "$.data.stagedLedgerAccounts[99].delegate" == "B62qnvzUAvwnAiK3eMVQooshDA5AmEF9jKRrUTt5cwbCvVFiF47vdqp"
jsonpath "$.data.stagedLedgerAccounts[99].nonce" == 0
jsonpath "$.data.stagedLedgerAccounts[99].public_key" == "B62qmKJxgh6h4i56hTXmkDL2Xpesm94NNm2Ev4ySSA4rCE2Z3JbfZhc"
jsonpath "$.data.stagedLedgerAccounts[99].username" == null

duration < 1000

# 
# Ledger hash query
# 

POST {{url}}
```graphql
query StagedLedgerAccounts($limit: Int, $sort_by: StagedLedgerSortByInput!, $query: StagedLedgerQueryInput!) {
  stagedLedgerAccounts(limit: $limit, sortBy: $sort_by, query: $query) {
    balance
    balance_nanomina
    delegate
    nonce
    public_key
    username
  }
}

variables {
  "limit": 100,
  "sort_by": "BALANCE_DESC",
  "query": {
    "ledgerHash": "jx7buQVWFLsXTtzRgSxbYcT8EYLS8KCZbLrfDcJxMtyy4thw2Ee"
  }
}
```
HTTP 200
[Asserts]

# Total accounts returned
jsonpath "$.data.stagedLedgerAccounts" count == 100

jsonpath "$.data.stagedLedgerAccounts[0].balance" == 75000000.0
jsonpath "$.data.stagedLedgerAccounts[0].balance_nanomina" == 75000000
jsonpath "$.data.stagedLedgerAccounts[0].delegate" == "B62qqhURJQo3CvWC3WFo9LhUhtcaJWLBcJsaA3DXaU2GH5KgXujZiwB"
jsonpath "$.data.stagedLedgerAccounts[0].nonce" == 0
jsonpath "$.data.stagedLedgerAccounts[0].public_key" == "B62qpbZkvpHZ1a5nsTbANuRtrdw4YraTyA4nvJDm6HpP1YMC9QStxX3"
jsonpath "$.data.stagedLedgerAccounts[0].username" == null

jsonpath "$.data.stagedLedgerAccounts[99].balance" == 1693980.63775165
jsonpath "$.data.stagedLedgerAccounts[99].balance_nanomina" == 1693980
jsonpath "$.data.stagedLedgerAccounts[99].delegate" == "B62qnvzUAvwnAiK3eMVQooshDA5AmEF9jKRrUTt5cwbCvVFiF47vdqp"
jsonpath "$.data.stagedLedgerAccounts[99].nonce" == 0
jsonpath "$.data.stagedLedgerAccounts[99].public_key" == "B62qmKJxgh6h4i56hTXmkDL2Xpesm94NNm2Ev4ySSA4rCE2Z3JbfZhc"
jsonpath "$.data.stagedLedgerAccounts[99].username" == null

duration < 1000
