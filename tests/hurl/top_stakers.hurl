#
# Top stakers query
#

POST {{url}}
```graphql
query Accounts {
  topStakers(limit: 3, query: {epoch: 0}) {
    public_key
    username
    num_blocks_produced
    num_canonical_blocks_produced
    num_supercharged_blocks_produced
    num_slots_produced
  }
}
```
HTTP 200
[Asserts]

# total data count
jsonpath "$.data.topStakers" count == 3

# check data
jsonpath "$.data.topStakers[0].public_key" == "B62qqhURJQo3CvWC3WFo9LhUhtcaJWLBcJsaA3DXaU2GH5KgXujZiwB"
jsonpath "$.data.topStakers[0].username" == "Unknown"
jsonpath "$.data.topStakers[0].num_blocks_produced" == 22
jsonpath "$.data.topStakers[0].num_canonical_blocks_produced" == 13
jsonpath "$.data.topStakers[0].num_supercharged_blocks_produced" == 22
jsonpath "$.data.topStakers[0].num_slots_produced" == 22

jsonpath "$.data.topStakers[1].public_key" == "B62qrxNgwAdhGYZv1BXQRt2HgopUceFyrtXZMikwsuaHu5FigRJjhwY"
jsonpath "$.data.topStakers[1].username" == "Unknown"
jsonpath "$.data.topStakers[1].num_blocks_produced" == 12
jsonpath "$.data.topStakers[1].num_canonical_blocks_produced" == 7
jsonpath "$.data.topStakers[1].num_supercharged_blocks_produced" == 0
jsonpath "$.data.topStakers[1].num_slots_produced" == 12

jsonpath "$.data.topStakers[2].public_key" == "B62qrecVjpoZ4Re3a5arN6gXZ6orhmj1enUtA887XdG5mtZfdUbBUh4"
jsonpath "$.data.topStakers[2].username" == "Unknown"
jsonpath "$.data.topStakers[2].num_blocks_produced" == 8
jsonpath "$.data.topStakers[2].num_canonical_blocks_produced" == 6
jsonpath "$.data.topStakers[2].num_supercharged_blocks_produced" == 0
jsonpath "$.data.topStakers[2].num_slots_produced" == 8

duration < 100
