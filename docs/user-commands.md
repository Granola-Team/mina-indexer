# User commands

## V1 User commands

There is only one kind of V1 user command:

- V1 `Signed_command`

### Example V1 signed command

```json
// mainnet-12-3NKkJDmNZGYdKVDDJkkamGdvNzASia2SXxKpu18imps7KqbNXENY
{
    "data": [
        "Signed_command",
        {
            "payload": {
                "common": {
                    "fee": "0.01",
                    "fee_token": "1",
                    "fee_payer_pk": "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy",
                    "nonce": "14",
                    "valid_until": "4294967295",
                    "memo": "E4YM2vTHhWEg66xpj52JErHUBU4pZ1yageL4TVDDpTTSsv8mK6YaH"
                },
                "body": [
                    "Payment",
                    {
                        "source_pk": "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy",
                        "receiver_pk": "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy",
                        "token_id": "1",
                        "amount": "1000"
                    }
                ]
            },
            "signer": "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy",
            "signature": "7mXVTdqb2tdWx5VrjyWcLQHYVdtd7KNzUvqxgbAC7KGGHNE1YWA2fYhbN791cJz7RvyjGaZTnkJkZUN318bujSwUrW1fvSqB"
        }
    ],
    "status": [
        "Applied",
        {
            "fee_payer_account_creation_fee_paid": null,
            "receiver_account_creation_fee_paid": null,
            "created_token": null
        },
        {
            "fee_payer_balance": "1438690000000",
            "source_balance": "1438690000000",
            "receiver_balance": "1438690000000"
        }
    ]
}
```

## V2 User commands

There are two kinds of V2 user commands:

- V2 `Signed_command`
- V1 `Zkapp_command`

Example V2 `Signed_command`

```json
// mainnet-359606-3NKvvtFwjEtQLswWJzXBSxxiKuYVbLJrKXCnmhp6jctYMqAWcftg
{
    "data": [
        "Signed_command",
        {
            "payload": {
                "common": {
                    "fee": "0.0011",
                    "fee_payer_pk": "B62qpjxUpgdjzwQfd8q2gzxi99wN7SCgmofpvw27MBkfNHfHoY2VH32",
                    "nonce": "765",
                    "valid_until": "4294967295",
                    "memo": "E4YM2vTHhWEg66xpj52JErHUBU4pZ1yageL4TVDDpTTSsv8mK6YaH"
                },
                "body": [
                    "Payment",
                    {
                        "receiver_pk": "B62qpjxUpgdjzwQfd8q2gzxi99wN7SCgmofpvw27MBkfNHfHoY2VH32",
                        "amount": "1000000000"
                    }
                ]
            },
            "signer": "B62qpjxUpgdjzwQfd8q2gzxi99wN7SCgmofpvw27MBkfNHfHoY2VH32",
            "signature": "7mX5FyaaoRY5a3hKP3kqhm6A4gWo9NtoHMh7irbB3Dt326wm8gyfsEQeHKJgYqQeo7nBgFGNjCD9eC265VrECYZJqYsD5V5R"
        }
    ],
    "status": [
        "Applied"
    ]
}
```

Example V1 `Zkapp_command`

```json
// mainnet-397612-3NLh3tvZpMPXxUhCLz1898BDV6CwtExJqDWpzcZQebVCsZxghoXK
{
    "data": [
        "Zkapp_command",
        {
            "fee_payer": {
                "body": {
                    "public_key": "B62qnMKF7DwQLUAYtJXHwCF7R7w8prL1BqsVa6g4qKELgzkwYuhpGze",
                    "fee": "0.005",
                    "valid_until": null,
                    "nonce": "186"
                },
                "authorization": "7mXFQb4d1m5McuhGgCGdtUuZQWzXr5yJfSJKjQLHHq4PbjmSxQ5WniAuiquwVd9yeKSwbqkEbFXyyrFau1GXn8d37kqMKCCM"
            },
            "account_updates": [
                {
                    "elt": {
                        "account_update": {
                            "body": {
                                "public_key": "B62qnMKF7DwQLUAYtJXHwCF7R7w8prL1BqsVa6g4qKELgzkwYuhpGze",
                                "token_id": "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf",
                                "update": {
                                    "app_state": [
                                        [
                                            "Keep"
                                        ],
                                        [
                                            "Keep"
                                        ],
                                        [
                                            "Keep"
                                        ],
                                        [
                                            "Keep"
                                        ],
                                        [
                                            "Keep"
                                        ],
                                        [
                                            "Keep"
                                        ],
                                        [
                                            "Keep"
                                        ],
                                        [
                                            "Keep"
                                        ]
                                    ],
                                    "delegate": [
                                        "Keep"
                                    ],
                                    "verification_key": [
                                        "Keep"
                                    ],
                                    "permissions": [
                                        "Keep"
                                    ],
                                    "zkapp_uri": [
                                        "Keep"
                                    ],
                                    "token_symbol": [
                                        "Keep"
                                    ],
                                    "timing": [
                                        "Keep"
                                    ],
                                    "voting_for": [
                                        "Keep"
                                    ]
                                },
                                "balance_change": {
                                    "magnitude": "1000000000",
                                    "sgn": [
                                        "Neg"
                                    ]
                                },
                                "increment_nonce": false,
                                "events": [],
                                "actions": [],
                                "call_data": "0x0000000000000000000000000000000000000000000000000000000000000000",
                                "preconditions": {
                                    "network": {
                                        "snarked_ledger_hash": [
                                            "Ignore"
                                        ],
                                        "blockchain_length": [
                                            "Ignore"
                                        ],
                                        "min_window_density": [
                                            "Ignore"
                                        ],
                                        "total_currency": [
                                            "Ignore"
                                        ],
                                        "global_slot_since_genesis": [
                                            "Ignore"
                                        ],
                                        "staking_epoch_data": {
                                            "ledger": {
                                                "hash": [
                                                    "Ignore"
                                                ],
                                                "total_currency": [
                                                    "Ignore"
                                                ]
                                            },
                                            "seed": [
                                                "Ignore"
                                            ],
                                            "start_checkpoint": [
                                                "Ignore"
                                            ],
                                            "lock_checkpoint": [
                                                "Ignore"
                                            ],
                                            "epoch_length": [
                                                "Ignore"
                                            ]
                                        },
                                        "next_epoch_data": {
                                            "ledger": {
                                                "hash": [
                                                    "Ignore"
                                                ],
                                                "total_currency": [
                                                    "Ignore"
                                                ]
                                            },
                                            "seed": [
                                                "Ignore"
                                            ],
                                            "start_checkpoint": [
                                                "Ignore"
                                            ],
                                            "lock_checkpoint": [
                                                "Ignore"
                                            ],
                                            "epoch_length": [
                                                "Ignore"
                                            ]
                                        }
                                    },
                                    "account": {
                                        "balance": [
                                            "Ignore"
                                        ],
                                        "nonce": [
                                            "Ignore"
                                        ],
                                        "receipt_chain_hash": [
                                            "Ignore"
                                        ],
                                        "delegate": [
                                            "Ignore"
                                        ],
                                        "state": [
                                            [
                                                "Ignore"
                                            ],
                                            [
                                                "Ignore"
                                            ],
                                            [
                                                "Ignore"
                                            ],
                                            [
                                                "Ignore"
                                            ],
                                            [
                                                "Ignore"
                                            ],
                                            [
                                                "Ignore"
                                            ],
                                            [
                                                "Ignore"
                                            ],
                                            [
                                                "Ignore"
                                            ]
                                        ],
                                        "action_state": [
                                            "Ignore"
                                        ],
                                        "proved_state": [
                                            "Ignore"
                                        ],
                                        "is_new": [
                                            "Ignore"
                                        ]
                                    },
                                    "valid_while": [
                                        "Ignore"
                                    ]
                                },
                                "use_full_commitment": true,
                                "implicit_account_creation_fee": false,
                                "may_use_token": [
                                    "No"
                                ],
                                "authorization_kind": [
                                    "Signature"
                                ]
                            },
                            "authorization": [
                                "Signature",
                                "7mXFQb4d1m5McuhGgCGdtUuZQWzXr5yJfSJKjQLHHq4PbjmSxQ5WniAuiquwVd9yeKSwbqkEbFXyyrFau1GXn8d37kqMKCCM"
                            ]
                        },
                        "account_update_digest": "0x3009752B1DCA27FD1F249D05995898085E228640AB44F9CB8B62B08A149EFE06",
                        "calls": []
                    },
                    "stack_hash": "0x229FEFE380507A2DB9F09EE31C6C8B0B2DE1095B508F95FE17FA5D2BA7C6DB4C"
                },
                {
                    "elt": {
                        "account_update": {
                            "body": {
                                "public_key": "B62qjwDWxjf4LtJ4YWJQDdTNPqZ69ZyeCzbpAFKN7EoZzYig5ZRz8JE",
                                "token_id": "wSHV2S4qX9jFsLjQo8r1BsMLH2ZRKsZx6EJd1sbozGPieEC4Jf",
                                "update": {
                                    "app_state": [
                                        [
                                            "Keep"
                                        ],
                                        [
                                            "Keep"
                                        ],
                                        [
                                            "Keep"
                                        ],
                                        [
                                            "Keep"
                                        ],
                                        [
                                            "Keep"
                                        ],
                                        [
                                            "Keep"
                                        ],
                                        [
                                            "Keep"
                                        ],
                                        [
                                            "Keep"
                                        ]
                                    ],
                                    "delegate": [
                                        "Keep"
                                    ],
                                    "verification_key": [
                                        "Keep"
                                    ],
                                    "permissions": [
                                        "Keep"
                                    ],
                                    "zkapp_uri": [
                                        "Keep"
                                    ],
                                    "token_symbol": [
                                        "Keep"
                                    ],
                                    "timing": [
                                        "Keep"
                                    ],
                                    "voting_for": [
                                        "Keep"
                                    ]
                                },
                                "balance_change": {
                                    "magnitude": "0",
                                    "sgn": [
                                        "Pos"
                                    ]
                                },
                                "increment_nonce": false,
                                "events": [
                                    [
                                        "0x0000000000000000000000000000000000000000000000000000000000000002",
                                        "0x169279C8A80FB3ED0D77AEBF92D4910478AC3C4CF3DAA816F69C284BC72AE25C",
                                        "0x0000000000000000000000000000000000000000000000000000000000000001",
                                        "0x3685CFEBC3484CB96FE0D58E368E47772FF8D56482E8E45F999BFB0BECA94C1D",
                                        "0x0000000000000000000000000000000000000000000000000000000000000001",
                                        "0x000000000000000000000000000000000000000000000000000000003B9ACA00"
                                    ]
                                ],
                                "actions": [],
                                "call_data": "0x0A787503476DA8B3C93D971A0264258D447EDAD1B0379D97C42AAADC8F01F6F5",
                                "preconditions": {
                                    "network": {
                                        "snarked_ledger_hash": [
                                            "Ignore"
                                        ],
                                        "blockchain_length": [
                                            "Ignore"
                                        ],
                                        "min_window_density": [
                                            "Ignore"
                                        ],
                                        "total_currency": [
                                            "Ignore"
                                        ],
                                        "global_slot_since_genesis": [
                                            "Ignore"
                                        ],
                                        "staking_epoch_data": {
                                            "ledger": {
                                                "hash": [
                                                    "Ignore"
                                                ],
                                                "total_currency": [
                                                    "Ignore"
                                                ]
                                            },
                                            "seed": [
                                                "Ignore"
                                            ],
                                            "start_checkpoint": [
                                                "Ignore"
                                            ],
                                            "lock_checkpoint": [
                                                "Ignore"
                                            ],
                                            "epoch_length": [
                                                "Ignore"
                                            ]
                                        },
                                        "next_epoch_data": {
                                            "ledger": {
                                                "hash": [
                                                    "Ignore"
                                                ],
                                                "total_currency": [
                                                    "Ignore"
                                                ]
                                            },
                                            "seed": [
                                                "Ignore"
                                            ],
                                            "start_checkpoint": [
                                                "Ignore"
                                            ],
                                            "lock_checkpoint": [
                                                "Ignore"
                                            ],
                                            "epoch_length": [
                                                "Ignore"
                                            ]
                                        }
                                    },
                                    "account": {
                                        "balance": [
                                            "Ignore"
                                        ],
                                        "nonce": [
                                            "Ignore"
                                        ],
                                        "receipt_chain_hash": [
                                            "Ignore"
                                        ],
                                        "delegate": [
                                            "Ignore"
                                        ],
                                        "state": [
                                            [
                                                "Ignore"
                                            ],
                                            [
                                                "Ignore"
                                            ],
                                            [
                                                "Ignore"
                                            ],
                                            [
                                                "Ignore"
                                            ],
                                            [
                                                "Ignore"
                                            ],
                                            [
                                                "Ignore"
                                            ],
                                            [
                                                "Ignore"
                                            ],
                                            [
                                                "Ignore"
                                            ]
                                        ],
                                        "action_state": [
                                            "Ignore"
                                        ],
                                        "proved_state": [
                                            "Ignore"
                                        ],
                                        "is_new": [
                                            "Ignore"
                                        ]
                                    },
                                    "valid_while": [
                                        "Ignore"
                                    ]
                                },
                                "use_full_commitment": false,
                                "implicit_account_creation_fee": false,
                                "may_use_token": [
                                    "No"
                                ],
                                "authorization_kind": [
                                    "Proof",
                                    "0x22379C83C9690CCA565FBE205A4789B5AD1C641001AC9239216B18019B0F2347"
                                ]
                            },
                            "authorization": [
                                "Proof",
                                "KChzdGF0ZW1lbnQoKHByb29mX3N0YXRlKChkZWZlcnJlZF92YWx1ZXMoKHBsb25rKChhbHBoYSgoaW5uZXIoOTcwNDJjNmVjYjM5OTkyNyBkOTMxNTg2OTEzYzYzMTkyKSkpKShiZXRhKDYyODE4MGVlOWMzMGJiYTIgZTQxZTdjNzZmYTk4OGE0ZikpKGdhbW1hKGQ4MjY3MmEzMGMxZWU4MmYgYmI4NDkzODEyM2M1MTU0MSkpKHpldGEoKGlubmVyKDk4ZmU4ODUwNWI5YzIzNGMgYWQ3OTZhMmFkYjcyYmNlMSkpKSkoam9pbnRfY29tYmluZXIoKSkoZmVhdHVyZV9mbGFncygocmFuZ2VfY2hlY2swIGZhbHNlKShyYW5nZV9jaGVjazEgZmFsc2UpKGZvcmVpZ25fZmllbGRfYWRkIGZhbHNlKShmb3JlaWduX2ZpZWxkX211bCBmYWxzZSkoeG9yIGZhbHNlKShyb3QgZmFsc2UpKGxvb2t1cCBmYWxzZSkocnVudGltZV90YWJsZXMgZmFsc2UpKSkpKShidWxsZXRwcm9vZl9jaGFsbGVuZ2VzKCgocHJlY2hhbGxlbmdlKChpbm5lcihmZDdiY2FkZWQzZTNlYWM0IDYzYjAxMWY1YWNiMjNkMjIpKSkpKSgocHJlY2hhbGxlbmdlKChpbm5lcigyMDkyY2EwMGIyNTM2OTljIDA4ZjBmOTMzMzRlYzU3MDIpKSkpKSgocHJlY2hhbGxlbmdlKChpbm5lcig5N2Y3NzU1ZWI5NTg0ZTNiIGRmYTUwODBhZWFhOThlZDcpKSkpKSgocHJlY2hhbGxlbmdlKChpbm5lcigzZjRlMDcxMGY5M2VjMGIzIDM4NjU1MjdiNzVmYTA2NDgpKSkpKSgocHJlY2hhbGxlbmdlKChpbm5lcig0YTk1ZWY5Y2I3Y2JjZGU2IGNlODM4YTNmNTE1ODk4MTEpKSkpKSgocHJlY2hhbGxlbmdlKChpbm5lcihkNWQzY2I2ODM4MGM3ZjgyIDk5NGU1YzJiODNmYTBlOTEpKSkpKSgocHJlY2hhbGxlbmdlKChpbm5lcig2Y2E3NjY3Yjc3YjJjMzUxIDRlZjQwMWE0Yjg4ZDliZTQpKSkpKSgocHJlY2hhbGxlbmdlKChpbm5lcihlMTQxZGY4NjA2MzRmZGJhIGU4NTk2YWZjY2NlNDM3OGQpKSkpKSgocHJlY2hhbGxlbmdlKChpbm5lcigwZmY5OTNjYWYzYzA4M2Y1IGEyMjU3NWY2OWNkZTQyZDIpKSkpKSgocHJlY2hhbGxlbmdlKChpbm5lcihiNzhhM2NiMTM3NGMxNWE4IGM3YmZlMDI3NDQ2MWM3YzQpKSkpKSgocHJlY2hhbGxlbmdlKChpbm5lcihhZWJmZTFiM2UzZWRhNTk4IGIzOGVmNmU1YjM2ZjFlOWEpKSkpKSgocHJlY2hhbGxlbmdlKChpbm5lcihkMGE2OWRmYTZjNDU4MmUyIDZkZjBkNDdmZDg2ZmY2MzIpKSkpKSgocHJlY2hhbGxlbmdlKChpbm5lcig1MzIxNDYzMDA1MDg2Nzk2IGEyNzAwMDQxYzZiZWIyNmEpKSkpKSgocHJlY2hhbGxlbmdlKChpbm5lcihjNGI5OGM2ODQ5YTM1NWY2IGUxNmRlN2IxNzk2NmY3ZGMpKSkpKSgocHJlY2hhbGxlbmdlKChpbm5lcig1ODdmMDQ4MThlZjU1NmQ4IGMyMDEwNWJmYTQ3ZGY4MWMpKSkpKSgocHJlY2hhbGxlbmdlKChpbm5lcig3ZGNmMTAyZTU4MzdhOGUwIDNmMjViZGFmNGY1MTFhNjgpKSkpKSkpKGJyYW5jaF9kYXRhKChwcm9vZnNfdmVyaWZpZWQgTjApKGRvbWFpbl9sb2cyIlwwMTEiKSkpKSkoc3BvbmdlX2RpZ2VzdF9iZWZvcmVfZXZhbHVhdGlvbnMoNGZjNGVkOTZjZjE2MWMwNiA0OTcwYWNhNTQyZmEzNzE5IDBhMGM2ODk0NWQ4NjlhZWMgMjM1NjUzNTJjMjI4NDgwZSkpKG1lc3NhZ2VzX2Zvcl9uZXh0X3dyYXBfcHJvb2YoKGNoYWxsZW5nZV9wb2x5bm9taWFsX2NvbW1pdG1lbnQoMHgyQTIzNTYzNTcwODNERDk3MkM2OTVFNTAxNDg2MUJCQzYwQTlGRTc2QjNBMkYxMzkyQkY0RUY3RjI0MjUwRjdEIDB4MkQ2QTlFRTg5N0I4NkY1NkQ0QzFBQkM1NTZFNDZGMjdDMDczN0I3MENBMDkyM0VGMDgyRUM2OEZFODY3OTQ4RikpKG9sZF9idWxsZXRwcm9vZl9jaGFsbGVuZ2VzKCgoKHByZWNoYWxsZW5nZSgoaW5uZXIoMzM4MmIzYzlhY2U2YmY2ZiA3OTk3NDM1OGY5NzYxODYzKSkpKSkoKHByZWNoYWxsZW5nZSgoaW5uZXIoZGQzYTJiMDZlOTg4ODc5NyBkZDdhZTY0MDI5NDRhMWM3KSkpKSkoKHByZWNoYWxsZW5nZSgoaW5uZXIoYzZlOGU1MzBmNDljOWZjYiAwN2RkYmI2NWNkYTA5Y2RkKSkpKSkoKHByZWNoYWxsZW5nZSgoaW5uZXIoNTMyYzU5YTI4NzY5MWExMyBhOTIxYmNiMDJhNjU2ZjdiKSkpKSkoKHByZWNoYWxsZW5nZSgoaW5uZXIoZTI5Yzc3YjE4ZjEwMDc4YiBmODVjNWYwMGRmNmIwY2VlKSkpKSkoKHByZWNoYWxsZW5nZSgoaW5uZXIoMWRiZGE3MmQwN2IwOWM4NyA0ZDFiOTdlMmU5NWYyNmEwKSkpKSkoKHByZWNoYWxsZW5nZSgoaW5uZXIoOWM3NTc0N2M1NjgwNWYxMSBhMWZlNjM2OWZhY2VmMWU4KSkpKSkoKHByZWNoYWxsZW5nZSgoaW5uZXIoNWMyYjhhZGZkYmU5NjA0ZCA1YThjNzE4Y2YyMTBmNzliKSkpKSkoKHByZWNoYWxsZW5nZSgoaW5uZXIoMjJjMGIzNWM1MWUwNmI0OCBhNjg4OGI3MzQwYTk2ZGVkKSkpKSkoKHByZWNoYWxsZW5nZSgoaW5uZXIoOTAwN2Q3YjU1ZTc2NjQ2ZSBjMWM2OGIzOWRiNGU4ZTEyKSkpKSkoKHByZWNoYWxsZW5nZSgoaW5uZXIoNDQ0NWUzNWUzNzNmMmJjOSA5ZDQwYzcxNWZjOGNjZGU1KSkpKSkoKHByZWNoYWxsZW5nZSgoaW5uZXIoNDI5ODgyODQ0YmJjYWE0ZSA5N2E5MjdkN2QwYWZiN2JjKSkpKSkoKHByZWNoYWxsZW5nZSgoaW5uZXIoOTljYTNkNWJmZmZkNmU3NyBlZmU2NmE1NTE1NWM0Mjk0KSkpKSkoKHByZWNoYWxsZW5nZSgoaW5uZXIoNGI3ZGIyNzEyMTk3OTk1NCA5NTFmYTJlMDYxOTNjODQwKSkpKSkoKHByZWNoYWxsZW5nZSgoaW5uZXIoMmNkMWNjYmViMjA3NDdiMyA1YmQxZGUzY2YyNjQwMjFkKSkpKSkpKCgocHJlY2hhbGxlbmdlKChpbm5lcigzMzgyYjNjOWFjZTZiZjZmIDc5OTc0MzU4Zjk3NjE4NjMpKSkpKSgocHJlY2hhbGxlbmdlKChpbm5lcihkZDNhMmIwNmU5ODg4Nzk3IGRkN2FlNjQwMjk0NGExYzcpKSkpKSgocHJlY2hhbGxlbmdlKChpbm5lcihjNmU4ZTUzMGY0OWM5ZmNiIDA3ZGRiYjY1Y2RhMDljZGQpKSkpKSgocHJlY2hhbGxlbmdlKChpbm5lcig1MzJjNTlhMjg3NjkxYTEzIGE5MjFiY2IwMmE2NTZmN2IpKSkpKSgocHJlY2hhbGxlbmdlKChpbm5lcihlMjljNzdiMThmMTAwNzhiIGY4NWM1ZjAwZGY2YjBjZWUpKSkpKSgocHJlY2hhbGxlbmdlKChpbm5lcigxZGJkYTcyZDA3YjA5Yzg3IDRkMWI5N2UyZTk1ZjI2YTApKSkpKSgocHJlY2hhbGxlbmdlKChpbm5lcig5Yzc1NzQ3YzU2ODA1ZjExIGExZmU2MzY5ZmFjZWYxZTgpKSkpKSgocHJlY2hhbGxlbmdlKChpbm5lcig1YzJiOGFkZmRiZTk2MDRkIDVhOGM3MThjZjIxMGY3OWIpKSkpKSgocHJlY2hhbGxlbmdlKChpbm5lcigyMmMwYjM1YzUxZTA2YjQ4IGE2ODg4YjczNDBhOTZkZWQpKSkpKSgocHJlY2hhbGxlbmdlKChpbm5lcig5MDA3ZDdiNTVlNzY2NDZlIGMxYzY4YjM5ZGI0ZThlMTIpKSkpKSgocHJlY2hhbGxlbmdlKChpbm5lcig0NDQ1ZTM1ZTM3M2YyYmM5IDlkNDBjNzE1ZmM4Y2NkZTUpKSkpKSgocHJlY2hhbGxlbmdlKChpbm5lcig0Mjk4ODI4NDRiYmNhYTRlIDk3YTkyN2Q3ZDBhZmI3YmMpKSkpKSgocHJlY2hhbGxlbmdlKChpbm5lcig5OWNhM2Q1YmZmZmQ2ZTc3IGVmZTY2YTU1MTU1YzQyOTQpKSkpKSgocHJlY2hhbGxlbmdlKChpbm5lcig0YjdkYjI3MTIxOTc5OTU0IDk1MWZhMmUwNjE5M2M4NDApKSkpKSgocHJlY2hhbGxlbmdlKChpbm5lcigyY2QxY2NiZWIyMDc0N2IzIDViZDFkZTNjZjI2NDAyMWQpKSkpKSkpKSkpKSkobWVzc2FnZXNfZm9yX25leHRfc3RlcF9wcm9vZigoYXBwX3N0YXRlKCkpKGNoYWxsZW5nZV9wb2x5bm9taWFsX2NvbW1pdG1lbnRzKCkpKG9sZF9idWxsZXRwcm9vZl9jaGFsbGVuZ2VzKCkpKSkpKShwcmV2X2V2YWxzKChldmFscygocHVibGljX2lucHV0KDB4MTY3QUNDOTM4QTFEQTQyMDM4NkUzNUNBRkZCRDE2RjJDQUZGNTA1NTY1OTQ1MTc0MjE3MEY4NTYwMDJCNEY0RSAweDM1QjFGQUNDMkIzRDJGNEFBRkNCMjM5NDI5OTY3MzlDMzk5MUUyODBGRkQ1NURERjJENDMzNDlENEJBMzM2M0YpKShldmFscygodygoKDB4MTM2NEM4MkM4Nzg5MzQyN0EzQjU1MThDNTY2RkE4NTQwRDNGMzEzQUVCMzMyQjI0MkEyNjA5MkMzODAxMkU0MykoMHgyNTIzNkFEMEI1NTkzMzkxRjU1MTU2NDEwMjUxNkRDMTg0MDYzMzUwRTRGNTUzREU0QkYwN0JDMEY4RjQ2QkNBKSkoKDB4MjI3NEM2Q0QxOERCOUY4RDM1QTE1QkU5MEZFRjAxOTc1MEU3Q0EwN0QyNDkwOTY5QjlDQzJBRDNFMEI4QTIwRikoMHgzNzVBRkMwM0M5MjIyMTg2QjE2OTgzNDdFMUNFQ0I1MzQwODc4NTlFNDgzMTc4RjZCNjY0RTZGMTZDNUU2RDEzKSkoKDB4MjNGNEIzOTlCOTZCMDc2RUE5RDMwMTAxOTM5QzM2N0RGNzY5QkE2REVBMjI3RDc1NTlCQkM5MjBENDU4RTJGNCkoMHgzMjA0RDg0QjVEMzgzMzQ4NkQ3NTgzQzZDOEEzRTY0RDAxMjUxOEZGNTUxQzY0RkZGRTgwNTA2MDc3RDA5MTEwKSkoKDB4MEJGM0I0QzJEQzgwNTdFNjk2NDM2QjNFM0RFQjAzQzc2QjI5RjczMzQ0MDIwMkZCOEVDREUzRDY4MzBEQUQ3MSkoMHgyRkI3OUMxNDhBOUY0RDIwMDM0NzBFNjBFQzU0QUU0RDI1NUM3QTFGNzA2Mjk0OTQxQUFDRjk1QUVFOTE1NzM4KSkoKDB4M0VDODQ1M0JDM0MyREM0NzRDRUYzMjE2RUE2OUI1MTU5RkIxMUQyQTVCNzcyQzQ1NTE0QjMxQTA4NEQ2M0NEMykoMHgxNEM3RTVFNkUwRTIwODkxNDc2NDdDQjEzMTg1RUU3NzJCQTg3RjA3QzA4QkVGMUJDMjc3RTM1MTBDQTNERDc2KSkoKDB4MzdFOEI2N0ZBRTUwODNFQUM1M0Y0N0IyOUE1RThDQ0RDODA3QUM0QTAwNzY4MzkxOThDMjk2MzFDQ0UwRkE5MikoMHgyMkI5QTgzQTE4MkIxRDUxNDBENjY3MTExRjlBRDBGMDY4QTQ1QTY4QTJBMTg5QTQ0MTk2ODczMEJDQTRGQUZGKSkoKDB4MUUyQzA3NTZBNzc2QUNBMDlGNkQxNzA2QTQ1OTNFRDE4RUE2NEE1QjgwNUY5QkM2MDU4QzAzMTYwQjBCMzNGQikoMHgwNzg1MjBDREFENTlFNEQ2OUI5ODRGOTE1RDE4RUJBMTA4NkIxQzJEQzgwRDA5QTQxRDc5RUY4MEE4RjdBNDkzKSkoKDB4MDg5RUQzQUUzQjZGOTQyMDNERDlGQjUzRkFDQzk3OTRDMEI3QkQwODZGQzc4NjA5Q0JDMUNBQzJEMTBGMzJEQykoMHgxNjY3MEREMzY0OUUyODhBRjlBOEZBQzhBRjNCQThENDMzMDA4MDQxN0Y3MTI4QUU0RUFEMjFCRjY4REIyQURBKSkoKDB4M0Q1RjU3RjA4MTMzRUYxODMxMTRDMkM5RjVCNDZEMEJFMTEzRDhDRUVDNUYzMzc1RjVBRTkyNDJFOUY2QkREMikoMHgwRkM5NjEwNTEwMDNBRTgyNERBRTQ0MzExNzE2N0JCMDM3QjdFRDI1M0RBMUVDN0ExRkNBNzIzMDIxOTFEOTM4KSkoKDB4MEIwODIxQzRCNUZBREE0NjI4RkQyNDlDM0VCNjU3N0FEQzU0MDE2N0RGQkM2MEUwMTQ3MzE0QjY3ODIzNDc1MykoMHgzOUE4Mzc5QjRGQ0Y1NTA1RDEyNjlBQzcwMzc3RTc2NzBBNzYzNDI0NTc2MTk4QjcwNkQ1ODAzODc0NkU1MTUwKSkoKDB4MDlCQjZCNTQ1ODEyMkMwQjM2MTE1OTk3M0RFRjIwQjlENEE4NERBRjA5QzcxNjFEQzZGNzJGQjQ4QTlEMjc1MCkoMHgxQjFGNTUwMDEyMDM4ODRBQkNFOUEwQTkyNDNCNjRGODA2QkY3NkUxMUQ2MTgzQTIwQzc5OEQwMkFBQUJFODg2KSkoKDB4MjNDRTRBRkM2NEM4NTUxOUJGODlGQUY1NTY0RTk2RjhBMzUyQTk2QUY0QjAxNzNGREUwMzBBQTcxNjg3Nzg4NykoMHgyNzZGRjBBMTk5QzgzNDhGQkQ3NDVBN0RDMzYyODQ1NUJGQzEyOTkzQjhBQkM1OTMxNTQ1ODI5NzBFN0U5REFEKSkoKDB4MjBFRUEyNDY3NkJDNDczRkQxRDg1QkExMUE0QThCNDhEQTI1MzJGNDlFMjAxQzkzNUI3QTVFMkVCRTNBOUQzNCkoMHgzRjAzQjUzMUYwNEIyNEM2MjI0OTNDNUM3MEY0MEYxM0E0OTE1MTE0OUFCM0YyQTlGQzQ1ODczNkMxMjUzQ0M2KSkoKDB4MkNBRTc0MUU4NDEwNDA5QzhBODQxM0E2MUQ3Q0VDRUQzOTJEQjVEMjYxQ0JGQkM3RUE5OTgzQzA4NjJDQ0REOCkoMHgwRTBGMEQ4QzRCMDRFMENDRUYyQkE4OUI1NEZFQkI5NDUxODBGRTk3NTMzQTc5Njc1NTQ2MjA1MkZCQzZGODVDKSkoKDB4MThEQTc2QTc5QjA0NTQxN0Y3MTk2MDlGMkVEMTFGNkI5OUVFNDRFRjlFMjBCQjI3REE2MTU4REM0OTVBOUM3NikoMHgyRUUzNjY4N0Y4MjgwRDZFQzE4NzU4OEM4QjVDMkI5QUVCQ0NGODk2OEQ0NjQzRTUyNEU3MDdDRUI0QjI5RkI0KSkpKShjb2VmZmljaWVudHMoKCgweDFEMjhFMjFBQkU0NzRGQkMwRjFDNTNBQzhBM0IyMjUzN0ZGRjUzNDk1MkQ0NUYxMjlCOUQ1QzFGMEI2NUMyM0IpKDB4MDkxNjgzN0NDMTI1MEI3M0RDNDMzQUU2QTkyNzA1MjBDMzdCMUQyQUY0ODI2MjI3MEQ1RENBM0E2MUQyMkY3RSkpKCgweDIxMzUxNDQ0RjJEQ0U1RTI2NkYyMzlCRTRGNDVBN0MxQTI0MUNEN0I5MTAwMzZCNDI2NEQ3MzRFNUNEQjM4MEQpKDB4MjMyQTQxNjhBRUI4NDAwQ0VDNzlFMkQ5MkIyNjJCQzkwODEyOEUzRkQwMUFENDFGMjA1MEJBRkM3NUNFRThBMCkpKCgweDAyNEE0NEQxNDFFNTA0RTZFNDM0QjZFQTlBRjFBMDE2QTA3MDNDMzA5RDFBQ0FFQ0RDRDIxNjFBRjVCQkQ3RDMpKDB4MTRFRDg0QjMxRkIwOUI3NjE0NDc2M0Y1Njc5NThCNjUwNDA4NjIzRkZCNjMyOENERUI0NUEyNTZDMkVGM0ZGQikpKCgweDJEREZENTBEM0RCNkJGODcwNkYwOEMxNDlGNTFGNzgzQUFDQzM3NTg0MkMyOURGQjBFQkQ2OUI0MzAwRTRGMTcpKDB4MjA5NTE5QzBFRkU2M0E0RUQ3QjVCNjY4NTQ5NjcyODAyMUFDNzYzQzdFN0NBRDYwQjkzMDE2MjNFQ0ExRjNBOCkpKCgweDFGNTI1RDBCOUE0MzE0NENDNjZBNzZEQkUyM0ZCRUU3RDk0MDM0QTNGQjc4OEZCNDQwOTY1MEM1NjVFQzA5NDQpKDB4MjFDNTJBREY1MUNCQkEwOTIxRTNBRTBERkI1NjNCODk4OEYzMjYyQzdBQkYwNjEwMTM1NkE0REQxQTQ0OEExOSkpKCgweDFFMUZCQTk2MkFFNEM5QkI0RUUzODBBNkNFRkUzQTNENEFCQUZFQzM5NjU3QTAwREQyRTY1MDc4OUJDM0M0OTgpKDB4MjNFNzkzQjMzQUQwRDcyMjExREFGNDc4MjA4NjI3QzJCQjBFMjFEQTczRDdDMDVEMEY2QkJFMUU4ODQ1RjU4OCkpKCgweDIxQjM0NzgxMTZFNjg2NkExMjBENEEwM0RCQjI4RTJFRTk0MjY3N0ZDMTI5OTAxOUQ0QUIwQzdFQTdFRUVBRjgpKDB4MDNBMzczMEU2RDZDRjZEMkNBQTg2QjBFRDM1MEU3RDU1OTJENDdDMjkwNTk2NUM4OTE5RkZEMUEzNUMwMDgxOCkpKCgweDEyMTQ4RDcyRjNDRjIwRkNFNDA3RDNEOTUyMTlCMUE5OTU2MjNBOTgwOTcyMkYxMjY4NkI2RTEwRTMyREQ3MjgpKDB4MUVEMDlDREE2QTE2NDRFRkU2MzE1RUIzQjM4MzFBOTQ4NzFBN0M0REIzMjlFNkE5NjBGQTlBOEFCRTg0MDBEQSkpKCgweDI4ODAxRUM2ODI2QjcxQzdCRTM1REMyODk5NEU5MTE4Qzc1MEY5Qjc0RDc5QkNEMDhGMzMyMEE5OUZCRURFMjMpKDB4MTU0RDVEMjJBMzE1MzM2QjYzMzVCMzIwQzZGMjA5REJFQUE4QTYzNUQ3OUUyMzdDQzUxREM3RkM0MUVDQzhGNykpKCgweDI2RDM5RUZFQ0QzMTE5RTAzODFBQkQwQjEzRDhGODREMjlCOTlBQjlFRENBMTc1RTMwRDM4MDJCQjMxMDMyQzcpKDB4MkVCQUZDODZGNDYwN0IwNjExMjQ1NDk2ODM0MTkzRUU4MjgwNTc0MDc2RTgwNjQ2MDlBMDhGODQxM0Q1MkUyMykpKCgweDNBNjk5MDEzQTMzMkU5NTQxOTJGMjA5OUUxRTcyQTlFMzc3NTUyQTZFMDhCQUZDMEEyMDgyNzNCQTIyRTU0MTApKDB4MjE1MUYyNTc3N0Y5Njk1Njg5RDAwNkJFQTgzMzFCN0FGQ0IzQUQ4NTI1ODVDN0YyREY4RTE4ODIyOTIxMjBDMykpKCgweDAwNjEzNjI5RDJFQ0RCODg0NjlCNjhCRTYzQUFCNjQ2OURDMUY0NTEzODUyM0I2Q0FEQjcyQkU1MzJDMjREMDkpKDB4M0YyRDNCMkFFRDE1RTQwQzhCM0VDMDUzODRGNDBCMTUzOTZCOUJFNkFBOTU1RjA5M0Y2MkI3M0YwRURFRDFFOSkpKCgweDAzRUNCNTc3Nzc2QzEyRjA2OEI1MEIxMzNEOThCRTgxRTlFQ0E1RkIzNzg1NDg2Qjc5Q0NCODQxQkRDN0EzRDMpKDB4MDU1MkQwMzJDRTA2Nzc0QThDN0QyMjI1RkEwMDI0QzFGNUI5MTc3MDg4QjQ3RUU1RDBERUZGODc4N0QzMzcxOCkpKCgweDExMkRBQjRCMzREMkU1MjZDOTQwN0VCQUMyQUQ1MDBGOTFGQzEyOEI5MTQ1OTVCN0MwQjg5RjdCMzBDM0ExNjgpKDB4MUQ2QjU0RTY4QURERkZGNEYwQjAyRjdBQTFERjdFOEMyNzlBOEEyNzZGMENDOEFERjk4RUYyNEJEQzM0NUIxMCkpKCgweDJCNjI3RUQ5QTU5RDdGNThDQkQ4QzgwQzAyOUQxOEVBOTIxN0VCQjE5QTI2OTg4OUY1NkVCQUI1MzQ3RjVFODgpKDB4M0YxQkY0NjBEODhDMDFENzc1QTlDQkNBNEQ0N0MxMjY4NDdFNjY4NkU0MEE4QkZFRjlCODdGQUZDQjI5OTY3OCkpKSkoeigoMHgwQ0ZDRjYxRjk1RDVENjJFMTkzQkFGNTNGRTE0RjM2Q0NDRkE3NjY4RUM2MzE4Q0M5NDkxNkVGMjVFNzg4QkI2KSgweDMwNTc2MzM5MTY1M0NEMkZERDhFRDYzQjEyNUQ5NjAyQzFENzBENkE0RDg1MTBGMkYwOEYxQTU0RThBQjJDNzYpKSkocygoKDB4MTM3NDRFQUU3MzE3MjcxRjkwODgxNDVFNTI5NjAzNTIxQkQ0RjE4RjhBMzlGNTJDNUM5QjMxNEI2NzE2MjJBQSkoMHgxMzQ5M0U1OUYwQTgwM0JBQjI1MkU3QkEzRjJEM0VGREVERDJBRkFGMzcyRTc1M0EwRkEzNUUwNjlFQzNDMTUwKSkoKDB4MzJDRTdGRDY2RDY2QTY4MzgyMUUxQTg2OTZCRjU2NDdGMjIxMDE2N0IxMkM0MERFREE5QzFCODI2NDJBMTdCNSkoMHgyNzM3RTA1NkIxMkZDMkYxNjZBMkJBRDQwMDdEOEU4NzVENjQwNDZEMjg2RDVDMERGRjdDNUM5MURDMDU3NDI4KSkoKDB4MDZCMDEyNTFDNjhFNzdFMUMxM0VFOTgyRjlDNTc2QzBDNzQ3QTZCOEZEQUU0M0M4NTIxNjA4NjU5NUI4NzU5RCkoMHgxQzY4MTUxRjA3NjFEMDg1Rjg1MDVBNjBCMDBEMjFBMkM4Q0IzODFBODQzQ0E0QTU3RTNCNEJBMjhFOTQxNDhFKSkoKDB4MjkxNEJGNDU0OTRENzVFMkI3MjhGQzUzRjczREJFRDIyNDZEMTA5QTJFRjM2QjdDMkZCNDQ2RkQ1MjBBRkNCNikoMHgyNTM5OTg3OEJDNjU4RUE4QTM4QzgwN0Y1MzREQjRBREVCOThBNTk1QzhGQkI3RkZCNDVDODZCRjU4OEUyNDMxKSkoKDB4M0QzN0REMjMwRDlDMjNENTE4REFGQzRCRDQxQzUyMjI0OTdCRkUzODI1QzQ4OTdFQ0U2NUIxMzM2RDI2NURCMSkoMHgzRjdBRkJGNTMzRUI0NzU1RDU5MDJENjM5MDQxRDFERUI2MzI1NTFBM0NGOEM0OUM0NzNDMDgxRjJGQzdENjQ3KSkoKDB4MEU2MjBFQkEzN0UxN0IzNjU5NTVFRENGNUUzNzg2RDZEMDQ5MkEzRTk5NzVFNTVCODc5MTZBRDhFRDlCMTA1QSkoMHgzMzg3Mjg3QjREOTNFMDZGMUQxRDdEMkRENkYwNENCODBENDIxMzM4NTExMDI3MkQxMDgxQ0VGRDcwQ0JBQ0VBKSkpKShnZW5lcmljX3NlbGVjdG9yKCgweDEwODA0NzQzREQ3RkM5MzExQTM1N0YxRjVFNDNGOTlFNkI3ODAzNjE3OEQ4RjRGQ0FFQUUyN0MzNEQ1QUNGMjQpKDB4M0M3RjM4RjQ4MzM4QTVCRDE4MjY5MDZEOTYyOUE5QTdBNzE0M0UxMzVCOUE4QTBDRjRGMTAzNjJDMkI0QUFFRikpKShwb3NlaWRvbl9zZWxlY3RvcigoMHgxN0FEOTgwNEQyQkIyNzA2QjhBQjI4NTlDN0M0MUVGNTk3QzUwM0JEQzhDRjRFOTJBNjQ3QTk1NkM1NEM2NzZEKSgweDE2QkRFOEIzNjEzMTVCMEE3NUJCMjYyRjQxQUI3NjNFRUMwQURGN0JBNjIxQzc1OEMyQzA4RDBCMzkxRUNGNUYpKSkoY29tcGxldGVfYWRkX3NlbGVjdG9yKCgweDE5MTM2NkFEQTcwQTdCMTBDNDA3MTM5Qjg5NDczRDkyNzQwMzZFOThDNDMzQTE1NUY5QTI1Q0M1QTIyRDBDNEYpKDB4MEU1QjI0NDNFNkM1NTU4RUY3RUQzNkNFQzA3OTI1NzVCMUM5ODc3NkM1QUZGNUY5MjFEQkU5QUJBRkZBM0VEMCkpKShtdWxfc2VsZWN0b3IoKDB4MDY0NjZGRTRFMEQwNTg3MEUxN0JBNDA0NjczMkM2QUJFMDU1MUQxMkM0ODYyMDg1RjI0ODQyRDFCMkEyOERBQSkoMHgxMjdDMjE1M0E3NTZGQ0UwQjRGN0QxQkNCQzk4RkQ4NjEyNDgxOEE0NDA0QkJEMkQ5RjMzNEI5NDFGRUQ4NEY1KSkpKGVtdWxfc2VsZWN0b3IoKDB4MTZENDU4RDY0QTlBMTZCRjkyODNERTI1QUUwQTI5MjYxNzc3NzNGQjc3Q0Y3NTQzREQ0ODM3QTRFQzk4QTBEOCkoMHgwQUYxNzBCRDlFNkUzQTVENzlDQjAyNjA4MkY0MUE3NEEzODY0M0REN0Q4Mzc4QkU4QUQ2MzY2RDdEMERDRjkxKSkpKGVuZG9tdWxfc2NhbGFyX3NlbGVjdG9yKCgweDBGNzI4QjBDODcxNTEzODhFQzYzMzY3MkU1MzA3QjRDOUNDM0QzQzZDQUY5MjIxQkE0ODJGQUQzQ0ZBM0NGNUYpKDB4MTA0RjU1RkIyRTM0NDYwQkRDRTVBQUE1NEMwQjFGMzA1MTU4QzYwNzM0NDBFNUE2NTk0MjdERkNGRTkxRjVBNykpKShyYW5nZV9jaGVjazBfc2VsZWN0b3IoKSkocmFuZ2VfY2hlY2sxX3NlbGVjdG9yKCkpKGZvcmVpZ25fZmllbGRfYWRkX3NlbGVjdG9yKCkpKGZvcmVpZ25fZmllbGRfbXVsX3NlbGVjdG9yKCkpKHhvcl9zZWxlY3RvcigpKShyb3Rfc2VsZWN0b3IoKSkobG9va3VwX2FnZ3JlZ2F0aW9uKCkpKGxvb2t1cF90YWJsZSgpKShsb29rdXBfc29ydGVkKCgpKCkoKSgpKCkpKShydW50aW1lX2xvb2t1cF90YWJsZSgpKShydW50aW1lX2xvb2t1cF90YWJsZV9zZWxlY3RvcigpKSh4b3JfbG9va3VwX3NlbGVjdG9yKCkpKGxvb2t1cF9nYXRlX2xvb2t1cF9zZWxlY3RvcigpKShyYW5nZV9jaGVja19sb29rdXBfc2VsZWN0b3IoKSkoZm9yZWlnbl9maWVsZF9tdWxfbG9va3VwX3NlbGVjdG9yKCkpKSkpKShmdF9ldmFsMSAweDE4NkYzNkVDMkY5N0NFQzZDMjA0NkFBQjFBOTA0ODQyMDQwMjcwQzA0RTZFODU1RkE3OEFDNDI0NEU4NTI3RjEpKSkocHJvb2YoKGNvbW1pdG1lbnRzKCh3X2NvbW0oKDB4MDFGRTNEODRCMTE4RjI5M0JFNEE4OTI5NjE4RjkwNkFBNzBGMDcxRTJGMDEyREQzNzk0QUUxMjY2MDBBREU2RiAweDM0RUJFQ0FENEQ0Rjg5NDEyNDg0MkZBMzhFMkM3MEI5RDI1Qjg4RkJDNzU5MTVGQTg1NDAyNjk2ODBBQkM2RjYpKDB4M0JEQjQwRTMxNTdGRjNCNkUyMDQ1MEQ1QjJCMThCRTI2OTM2REY0M0E5QTE5MTZBNUUxNTc0QUQ3M0JFMUYwNSAweDEwNjYwM0UxOEQ2RjU1OTBBNDM3RDdEREE1RjcxNzIwNjAyNzRFMTk5OEM2NTRDMUVBMTg5OUY3REZDMjE1OTApKDB4M0U3NzQzN0NDMjREQkQ2QjNGRUI4OThCQzZGQ0Y4OTg5ODFDOTk5MkFBMTI1NTFCNzdERjc3RURCRTdBNTI3QSAweDE0N0NFMTQ5ODk4MzVERTBEMTM0QTJGREI5QUI0RTUyRDAzQkM0NjI5Njg4REFBRTNCOUFEODg1RDE1MUYxODYpKDB4MkMzRjQzQkQzNUQ2MUM3NzA0NDU5QUE0NDZGNzkxRDcyODY1QTc1RjZEM0I1QzgwQzgzQ0QwQ0I1OTk4QjhCNCAweDI2RUIxRjhENzEyOTVDRDg4NkQ1OEU3MDY0M0UxNDAxMkZDMDk2ODQzNUJCMjk1MzEzOTdBQTY2NTNCRTZBNUIpKDB4MTA5QjExN0M5MjcxMzAxMjI0RDQ4Nzg2MTk3MjVDQjY4NkI2RUJFREVFMEJEREQ2RkE2NEZFQjRDREI4QjE2QiAweDFDODM2MTQ0QUQ3NDczM0NGOUVCODQzNkM1MTc3QjRGOTdENUJBNzUzODBEQzkxRjFDREUyRjJEMkJFMEY0MjQpKDB4MEZFRUM3NTg3MjI2MENBNDVEOTZCQkNGNUY5REY1RTNCREQ2OTcwNkEzMDM1QzQ0NkY1NDJFNEU2OTMxRTU4MyAweDJDMENDOTVBMDM1NTc3NEVFRDMzQjYyN0RERTM2MzNBRkM5RjBFQzM2MUNGMDRGNzhDMDdCMEVCMTVCMTlFRkMpKDB4M0VGNEQwQTg4QTJBNkVEODQwQ0E3NzY1MzYwQjMzNzI4Mzk2MEQ3N0EyNjRBMkM3ODE0MEE3QTk1OTkwNzAwRCAweDM1ODM4NTdCOTdCNjFBQTQ4RkVCQzZFOTQyREI4MzNBRjVBQkNFQUU5RkM4MTBDNTIwMEFBODIxRjkxQzkyQkMpKDB4MEY2OTlEQzAxODkwMkFBNTY4NzdCRTBBNDgxNzAyNTRGODc2RDg0ODkxODdDNTg5NjI2RDcxRUUzREJGQ0UxNSAweDI2RDZFNkExOTZBRTg3OEQ5REIxMkJDNDFBMkY5ODJCNzU3OEYxMjI1Mzk3QTQwMUVBMTcxODQ4OTkyOTJFQUQpKDB4MEZCNkI5RDExMkJERjczMkMxRDk0ODJENTI4ODc2MTNBMzlGMTY1NzQ2MEE2MTIzQzRDM0E4N0M4OUEyNjE2OCAweDIxNzg2RDQ4OUE0OUFBRTk2NTc0Q0VFMUNGMTI4QjM4QTU0MzY0ODlCRUIzMTBFODM3RDZBRTAxNzMxMzM3NDcpKDB4MDg4ODk2NjVGNEE2NjdBQkE1MzE4QkQxNDA4ODEzQUQ0NTVGQkY4QUQ1MjFFQzZGODY5RTc2OTMwMzI0Q0VCMSAweDE2MjgwRUZFQzJDODZEMUUzQ0RCNzdCMzFGNzI4RjZBMkJCOUE2MTE1N0NGOTE0NzZGNjIxMEJGNjAwOTg5RDApKDB4MzZDMDk0NDEyODlBODAzMDlCNERDQ0U1MTM0QTcwREQwNTYyRkZFNjEzRThCQjI2NERFQzQyOUVGMzM0OEFGRiAweDIzRURENjVDOEIzODc5NDIyMTgyMjM2REM3QkQxODgyOEFBQzdENzU4NjhERTAxQzA2NzkzMjdCRTNBRjk4Q0UpKDB4MEQ1QTMzQ0Y3OTQzRTdEQ0M5MDBFNjkwMTNDNkQzQjMyQkY5RkQ4MTYwMkJEMDQ4MkIwOEU3ODJGRjRFQUVGMCAweDE5NkY5NEVCQTA3ODc4MTgzMDEyQzg3Mjk2NUQ3MUJCQkIxQzJDODA3REJDOEFFNUZERTk1MzcxRjA1RTZDODQpKDB4MkZEMzc2NUExMzgxQkNEQjM3RkVBNDAyRENBNDM3RDlDNDgzNDY2QkZFQUM3OUU1MEUzRjExRjM4MEQzQkRDMiAweDAxRkUwQ0ZBNEZDODRBQjg0ODcwRkI0OUUwQjY4MkU1RjMxNjg3RjkzQTlFQThCNzRDMzI0NEI1MTJDOTYwNDApKDB4M0QxMzZDRkY2NUIyMEJBOTg0OUNFMjhDMzQ1QUY0RDI1NEY3RDQyM0VFMzQ1QzdDNzMzMzkxNkY1Mjg2Q0Q0NCAweDMwNEUyNzk5NkQ1RUU0MEU0NDdFMzlBNTAzRDI3RjkyRjMyQUE1NzcwRUYyOENCQThBMUExNTdFNjQ3QUFBNDUpKDB4MzU2RkYzQzIxQzQzQTdFNDExRkYwN0NGMEQ3OEI5OURERTczMzVGNkMzMjY1QzQ2OEQ2QzJBNUQxNkJGMUVBOCAweDE2ODlDRUM3OTQ0OThGMUY0MDZBNjJGREVGRjE3NkI3REFGQUE2QzkwMTc0RUMwRDZFRDUzMEVCMjc0RkIxQTIpKSkoel9jb21tKDB4MzM3NjE3OUUyNTk1MjEzQjEyMEMwQjdCQTVFQjFERUM2QUIyOEUxRUNGQjQwNDZFMEQ1QzZDNkNFRDUzMTc3MSAweDE3OTBCN0E4NURCQjAzMDM1MTA2MjJDQTUyODM5RUUzNTM3RkU2NTFGNDUwOUIyNjFGNkQwRUVGMUE2ODQwRkEpKSh0X2NvbW0oKDB4MTY0ODc3MDIxODc3RDhEREM0M0Y4RjM3M0Y4OTVDOTlCODhENUQ1OTQzMDBENTIyRkJBQjAyQ0VBQTYwNTRCQiAweDM4ODVGQ0I5NzdFRUNDQ0YxRDZCQTk3MDIzNDFDQ0ZBNTRCRTFEOTQyNENEMjg4QzI3NjhEOENFN0UyOTYzRTEpKDB4MTg1MTFGNzA3OTA2MTYxNTMzQjc4ODM3RjhFNzg1NjNENDYzQzc5N0FERDY2RjE0RjNEREU3NjM2Qzk2Q0Y0MCAweDJBMENGNDY3QzM3QzNGN0EzRkFGNDdERjk0OUEyMjNFNjkyNURCRkEyRTBBMkNFMTJBOUM0RTJDMTJEOTQ0Q0QpKDB4MTY2QkRGQ0ExMjUwQ0JFNjQ5QUJCMDYyNjIwMTkxMDEzMUZEQkE0NzdEN0Y1NEVGNEMwQTQ5MDAwMzdDMkM4OCAweDEyMTk5MEE4NzE5RTlDMjIwNTY5MUYzMjNEMkQ5RTNBRjQzMTIwNjlDNjIxOTEyNjhBOTZFQjE2OTBFMDhBODYpKDB4MjM2MUQ3QTM2MEYwNkZGQjU4QzYyMEM2QjVBRjVGQjkxMDgxM0Y2QjBFRTA2QUU3MjAxRjEyMUEzNDc0N0FGNyAweDI2RkQzRTc0OTYyN0U4RjFFNTUxN0NDNzI2OUMyNUEwRDU3MURGMTQ3MTBDQjI1M0IxMkM3NjE1MkU4MDBGOEYpKDB4MkJCQjg0NzRDMTU3Q0MwOTEzRkYwRDdCMTBFNzNFQzI1MEY3OTg2REU4M0Y5QjVDQjE3NUNCMkY1RUQzNjZGNiAweDBCQjIxRkRFQzMxNkMxQTJBMkEyQTQ3MTlDMDQ0QkZGRTcyMTg2QkExODREODc3Mjg4MDBBMzM5REVCMjQxNEEpKDB4MURDQjE0NEQ1Mjk0MUI2Q0I0MEVBMTc3ODJBMkFDN0FCNDFBQ0IxODU4NjA1NTgxOTZBQjk5QkRGRDFEMjBCMyAweDM1ODU0NjgxMzZBMTFFNUZBNkNBMTYyNjM3NjdERDhCOTI3RTc5OTc2RjNERkQ2MUFDMTU4NDIxRDA1RTRCNTUpKDB4MDIyQUY0NUI2OEQyQzIyMzNCNTFBNDQ4REM2MTU4OTY1RUM3NkEzNzgwN0MyNzc3QzMxNTI4MDE0RjA0MzE4OSAweDMwNzIwNkI4ODA2NjJDMEM3ODIwMDJGRDdDQTI5MTg1QjhCQzhDMzQwMDgzNDczMzdBMzQwNUZGMzJFNUM0MkEpKSkpKShldmFsdWF0aW9ucygodygoMHgzODJGMUQ1MEE1ODNCMzJGNzcyQUVEN0E1NUQ3MTA4Nzg2OUJDREM2Rjc5QTQ5QjgzQ0I1NTFFNUU4M0MyRjM2IDB4MDI5REIyMzNBMTBFMjZFMkIwRjQ2RTBEMDQzNENDRUYzQ0JBQjlEQjM4ODY3MUQ1NkI3MzYxODRBRjlEMTJFQSkoMHgyQjBCRTk5RDI3MTA4NDdFODg4RTNCQTVEMjI1QzFCMENGMEQ0MUJDRkVDNTc4QjUzMkZDNTVEMDc4N0M5MTg4IDB4MTY1NDdDNEJEODYwMUY5RTlERUI3REFDNzkyNTI5N0RFMDUzMzBDNjBFMUMxRUYwOEVBNkM1RjAxQzJENUQ1MikoMHgwODBEOTg0NkExQ0Y5QjA0QURDNDcwNTI4NkU5QkI2RTgxMzI2NUM2NDIxNTU2MTZCOTZGQzE0OEVDOEJDRUJGIDB4MDFEMDRDNzU1N0M0OERENjQ3MjdDREJFNTU2ODYzN0RBMTBGNDlBQzc5QkJFMTA5MUFDNjYwOUFBOTE0OEY5OCkoMHgwMEEzM0RFMzI4MTc4MDcyMTIzMDMwNDdCODlCMEQxRjhGREE5QTFFQTdFNTRBN0M1ODdFRURFQkFGNUQ5RTE3IDB4MDVFQjI4MTZDRjM3ODQ2OUU0ODg1NDE5NTkzMUM2NkEwNURERUU4MkY4MkYxODhDN0E5NTlENzBFQzNBQzVBQSkoMHgxQTQ1QUNCOUE4QTM5Q0U2RkMwMkMwRUMxNzI3QUNGNzhGOThEQzhFMjhFRjQ2OTc5QTg5OTczM0IyNDI3QzIwIDB4MkIzN0VFMDlFM0Y2Rjg0NjQ5NjU1NDU0RUQzQjEwQTA2OEFCQzZGRjE2ODIwNDdDQzJCNzY1MjUxNkRGOEYwNykoMHgxOTVCNTZEODMzNTYxRTMxODY4NTAxNjM2QzI2QTFGRDg0NUE4QTMxQ0VCOUIwRDgxNTYxODI2N0Q3NTdENDYzIDB4Mjk4RjFBMDFGNjhFNjc2ODRCM0NCRUI3QTlFMDEwM0YwNDM1MThDQkE1MUFGRjE3ODk1Mjg4QTUxQTRDQkNGRCkoMHgxRDlFRTFFNTBEQzU5ODY5QkEyMjQ1QzlGMzUxNjY2MjNDQTI0QkU0OTM0NDk4MUFBQkE4REY2MTUyNkQ2RTZCIDB4MTNDMjU3MDIyOTYwQzJBOThFNTE5Qzg2OTgzOTc5MTZCMURCRUNGMzY0ODlCODQ3REY3N0Y4RDI4QUNGNkQ4RikoMHgxQTNEMjM5QjEzOTNEMzc1QUIwOURCRUY2MDI0MDkwQUNFMzkwQ0IyNUEzMjhBQkU3RTYxMEZEMkZCMDMyMkZCIDB4MkU0RDRCQjY3OTEzNDUyNkEyNDIwOTc2M0U3RDU4MDY5NjM4MjkwMjEyMzI0NkY0NjdFQjhDQTc2ODM2ODYyMykoMHgzQkU4RDA4RjQ2N0YzNDIxRDE4NDc3OTNFMEY3NjBGNUM0MDIyNDZCMTNCMTc2RDI1OTg0QkU0N0YwM0M5NzIwIDB4MjNBOTE1Njk3NEUxRjhCQzA3OEM5QzM0ODA4NjUxMjBCMTJDREUwMjU5NDZEMjM1QjcyODM0RDc5MTVDQjYwRCkoMHgzRDQ0NkQwMDA4NEY5RTg4NEY3MjY0QTA3MUFDMkY2N0RCQjY1NEQ0NDE2QkY1MzkwMjIwNDkzNEUwODkwMkVBIDB4MjgzQjgwNTU2RTIwMjQ3MTk1MDVFRTc4NzJBQThFRUZGMTY3OUY3RjM3OTM5QTUyNURFREE3QzMwMUE5MUYyOCkoMHgwRTJDMTU2NzAxOTFDOTk5ODUzNTc3RUM1NDI4QkVFMDVGNDhEMjIyREE4NUI5ODk5NzNBQjg4OTg5QTc1NzQwIDB4MzEwQUQ4MDkzRTI4RDU4NjI5MEFGMDcwODcyQTA5MTc1NDE2Qzk2RERDODA4QkI3OEI1MEQ4MDREMDZBNUY3MSkoMHgyMUU0MUYwRTYxRTg5RUI1RkU3M0I4MDVFMEFFNEQ4OTcxQTE5NjRFNERGRTVDRkYxRUUwNzJFMkMzRTRGNDgyIDB4MjM4MkNERTc3N0IzMUFCRTVGQUU2MDAwNzZGMTAyMEMzNzA0MUU5NjE1MzYwQjNFMkU5Q0QzN0RCMzhCNjk0MikoMHgwM0U4OTAzOUUxMjc1OEZENzI0MUVGNjA2N0ZGMTY2Qjg5QTY0OURDNzE3MzM4REEyQjA0M0RCMDgzNzMxM0ZBIDB4MDI3MzU2NTIwREM0MzBEODkyNTc4MEUzNTQwMTE4OTkxNjQyREJDQUUyMTMzNEEwQjQwNzZCRDVERDFBQTcyMCkoMHgyNjBFQzU5OTQ0QjVENEYwMTRCNDI0OTgwMzAzMzU4MEFGMkNDRDZDMkFDMEFDODE3NjIxNjc5QUIxQjI2Rjc0IDB4MkVCQTlDMjkzNkYwQzNCMTU3RTYxQ0JDN0VCODg0RjkyRTRDNUEzOEQ0NDU5NDkyNzQyMjFBMjAyMTU1NkUxMikoMHgzMTM5M0U2QUUzN0M5NkFEQ0NCNDY3QTFENkNENzM5NzQ5MTkyRkVDRkNENDk2ODVDMjg4QTlBMTEzQzY1RDAyIDB4MzY2MkYyM0VBMDVEMDE1RDY2NTA4N0ZBRjdDMEM1MzE5MDAzOUNENkNGOUREMDlCNjcwMUE4ODBCRkU2MjJGMSkpKShjb2VmZmljaWVudHMoKDB4MkE3NTk0RTc1M0ZDQjI0NUM4QjYzMjJGRjhGRUVEQTY1QzY1NzUwNDk0NkZERkI3QkU0QUMwMzcyNEY5QTI1QSAweDE2OUYwRDc3RkNFNEIzMDhCRjIwMzkzMDkwRDY5RTlERDgwMDA5MzMzMTk2RUNGQ0Y1Mzk0MTg5OEI3NTUwNDQpKDB4MjJGQUM2OUM5QUNGMUY4RDQ5Nzc2RTJBQUFEQzg3QzM3MkU3NzgzQUQ4ODY1NDhFNzQ5OTAxOTBGMTA2NEU0RiAweDEzQURFNjExMDY2MzhGMUUzRjBEOEI2RjFBQ0Q3Nzg0MDM4NUIzQjUzNzE5RDFBMzEyOTY2OEVCQjAxRjhDOTgpKDB4M0NDNjUwNUNDM0I5RUNEMkU2MTJDQkVBRTFCNDA3RjNEMDY3MzM3RUY3NEEwMTQ1RUQ4Q0Q1MUQ4Q0UyOUQwNCAweDMyMDk5OEY2QTJGQzBFOTk0ODAwMzYzMjkxNzQwNEIyNTcxNTBERkM3NjU4QUY5Qjg4NzUwREEyQTE5RDcxMTEpKDB4MjA4MUI5MUQ3Q0EzQ0Q2MEIwMkM2MEM1QzI5OEY3NDQ0ODQ2QTlGQzZFQTdBOURFRTIzMkE0MjZGRUI3NzIxQSAweDE5Qzc1MDk5N0MzREMxMjk2QzVENjdFREJFRjAzQzYxMUZGMUIxMkQyRkVEQzQ2QzFBRDY5RUY2RUREMjE2NjcpKDB4MUQyMDA3MDFCOUQ3NjNGRDU2RDQxRTlFQzgxOUE3NzI1NkVERTQwNUFDQzdFOUQ5Q0ZDRTAyNTE0QTM1M0FENSAweDA3NUI0MzJGMUREMTU0MTFGQkQyMEM2RDVCQTlDRTY5M0JGQkFEM0U4Q0I3MzhFQkNDNjFBRUY0QUM1NzBFRkMpKDB4MjBCNTJCOUMyOTU4M0EzMzdENDU4Q0U3OENDNjI1MTZDOEE1MUZCQUZGNDFCOUExNDI4NjVCODFEMkIzMkI4OCAweDBDRDZBN0IzODM3QUUxMTlGNjQ2RTY2NERFOEUxQkI2NzRBOUFGRDMzRUIwMDlBMDA3MDM5MzJFQTNFQjk2MDYpKDB4MDQzMUZDNkFEM0VEMUY2RDcwRjJEOTRCRkFFNUJFOTZGREQ1NzY3MzUxNjQzNDdDOTEyRDZGMjNCMTU0NjZBNCAweDEwNTcyM0Q2MzZBRTMzQkZCNkRBQUE4OTkwQTlDMjU5NjFGRjU1ODlBOUZDRjBCRDA0Q0E0RTQ2QUY5RTdBMkQpKDB4MjQ2NDk0QzM5NkJGOEJGQ0RDMTk0RjNBNDY5N0RCQ0VFNzkxQUIyMDUxMjVBNjk1M0QwNDNBMUJDQkI1RThENiAweDBBQkEzOUYyRDI3Mjg3MDQ0ODc3Njc5RUU1REQ5OUExRTE2NzdCOEQ2OTFFNERBN0U3ODMxMDQ5QUVFRDYyQTYpKDB4Mjg0OTczRTQxM0Q0MjZBMDg4RDM0OUQ2RTZEM0QyNUVDN0UyNzhFRjVBQzAwQjE0MzhDNjgzRTY0MjYwRTE5OCAweDAyRERERDM3ODU5MDEzNjEwQjE3ODJCNDlEMDhENTlCMDhGRTU0MDUwNDNGNzlDMzRGMEVCRUNDMEMxMTE2QUEpKDB4MTUzOTQzMjhDRUI0MzMxQUNDMDJBMEI4RUM5REFENDcyQTkxM0I5MDI1NDQxRjhGQ0U4MTAyQzFENkYzNDE1NiAweDA3MjNGMzgwRjdENkM2OEI0RTEyNkY0N0Y4MDZBODE1RjE4OUJCNDI0MEZDQkQ0QUUwQTY1MkY2NERBN0I0QjIpKDB4MDE1RTE1QUYzNzE3NjdFMjQyNTEzNEVCNjZDNUQ4NDUzNTlCNDczQ0ZFMzQxRTIyN0RBNUM0MDdGRjdGOTcxQiAweDBCMUMxMjNGNzZBNjdGQTE1RkZDRjM3MEFGQ0RCNjU0QUQ1RjFFQ0JCODg3RUI0N0YxQjMxNEM2MjQxRUIzOEQpKDB4MDIwQTJFMTVBRTQzQjZENUVCQTMxOTMyQkMxQThFNjAzMzI2OURFMkEzNTNDRUQ1MDNENjI2QjVFNzVDQ0JBMSAweDJCRUYzODNCRkQ5MjcwQUE5OTJEMEI1MkQxRThBQ0U4MEIzQzE2MEE0OTQ5RDZBNjUyMTFBQUJGMzgyRjFBMzMpKDB4MTk1N0Y5QTVFQkRGNDQzREU4QTg0Q0IxOEVFRDVBNTY3NzUwRTFDOENEREUyNzBFQTg5NTQxRTFGNDc2MTYxMSAweDI5QkFCMjhFQUNCNjgyRTcwODBBMTA4NDcxQzhDQzUyNEQyMTg0RjJEMDdEMjEzMjNBMDVCMDdCNjQ0RkQxRjYpKDB4MUVBQzYyRkVCNTY1RjkxNjdGRjQxRDMyNjVGMTY1REU4QTlDODU3MTFFNDE2QTA1QTQ0NkE0NkQxNDU3NDE2MiAweDM3NDgxRjc0QUEzMTc4NERDNUU1MTU0MkVBMzc5NTc3MDI4MTE0OUFGMTFBMUVBOTIzMzM4NjIzNTFGMzMyNTcpKDB4MTU1NDJCMkY5ODlGQUY1RTRDQzE0NzIzQ0M0RDUwOUFDMTk2Nzk4Mzc5REEyRUEzMUJCMzAxRDFEQzc2OTIxQSAweDJBRDFCRUQzNzYzRDJBMjI4QTFFOUUxMzEwNTM0NUU3Rjk3RkY1ODY5MzA0M0UwODgwRkVBQUMxNDIxN0Q0M0IpKSkoeigweDM2NjY5MjQ4RjdDOTUxNEU5MDIzRTY1QjhFOEQ4MjNBQzk2NDNEMkQwQjBBRDg3NzJFREM0NDFGMjI3ODM1QjcgMHgxREZGMjQ4NDdEMzdCMzBFNDM4MDYzMzg2QkE3MjZBQkVFQkFDQUIzMjUxQzEyNkM5ODlGMjEzODVCQUU0RkVFKSkocygoMHgxQzEzNDUwQTBDQzgyNjlDQUFBRDI3NjNEQjdBQTNBNUUzQjFCNDE4NTkyMkEyMDBFQTI3NDhEMUIwMjM0OERBIDB4M0MxQTU4RTM2RjFDMDA3Mjg1RjNENDQ4RDJCRjFFM0IzMzcyRkIyNzc5Q0FGMEFGMUI2QUQxNjREMTUyOUFFOCkoMHgyRTgyN0U2Qzc4Mjc1NEI2QTczQ0MzRTFDMDQwQkMyOTNCQjA0REU3MjM2QUVGQTIzQkNGQUU0MjdFNjU2NTI3IDB4MDIwMkJDQTdEQjBCMEJDMjUyODU0ODA0OUYzQzY1OUQwRUQ4NTYyMkQzMzUzMDM3QTY5M0FBMTdEQkM0OENDMCkoMHgxMjBBNkJBM0I3ODcyQzk0OTU2ODI4QUUzRTk3QkU5ODlFREIxMUY5NDYzQThBOEIzOEQ1RUJEQzYyMTQxM0M5IDB4MzY0MDM2QUI0RkE4MzE5MzE5RkQyOTY0NzU0MjQ4OEMyMUZFMEMwODU4Njg0ODJBMzgyODhFNjU2MjI0M0YzRSkoMHgzRTkyRTZFNUU2OUMwREYxNTU3MzkyMjlBOTRFQUVFOUJDNjBEQjU3MDNGNzhGOUZERDgwMDgwQ0ZBRjg4OTQ5IDB4MTg1RDRDODY3OEQ0Nzk1MDY3MDNCODE3RTQ0NThDMjFDMjEwNDQzRkUwRkY3QThGRjcxOUYyQjk0NjY4RkUyNSkoMHgwRDFBRkFDOTZEMjA1NzBBMzk4QTBERDU2NTVERDlFRTc1QzJEOTgxOTYyQ0MyOEZFRDNCMEJBRjVEQjJDOUNDIDB4MjMyOUUyRDJBQkQ0MkEzNDU0RkM0Nzg1QUUzM0ZFRDFGMkQ5RUQwNDhCOUY0RjQ0N0M3RTlBNUEzQ0NBNzZBRikoMHgwMDE3NjVGNzI0NjlEOTg3ODYzMzYwQjI2NjhDRUU5NzQ2QkM5N0M0OTEyNEM2NkNCNkQ0MjM5RkY0QjE0ODBGIDB4MzE0NThFRkZCNEY5MUQ2NjhENjIwQURGRDM2MDc4NUQ1QkYxODhCMzU2RERDQUE5MkZDRDM2QjQ0OTIzQkRDNikpKShnZW5lcmljX3NlbGVjdG9yKDB4MjdGRUExMUQxMjM5ODc3NTc5MDk5MUNFNzdFOTRBNUEzN0Q1RDhEQjU5Q0MxMjM4QTA0OUI5RDEwQkY5M0I2OSAweDFCODVCNkZGNTQzMjZDMEU1MjM5ODU0RjAxQTZDRjlFQTIwMTA1QzY5NzM1RDYxMUY4QTJCOUE4RjQ5NkU0QkMpKShwb3NlaWRvbl9zZWxlY3RvcigweDEyRTQ0OTI2MUMzQkFGQzE4RUI0QzE5OTRGNDA4MEI1MDk4MjQ2MEJFRTRCNjFGRkNBMzBBQzVBOUZCQjI5QTAgMHgzNzZFQTlGRjQ5MEMxMTk3MTY4MUJFOTBFODlBNTE0RDVBRUY5Q0RGRUNEN0E1NEE4MzA2ODgyMzk0ODdCMzE1KSkoY29tcGxldGVfYWRkX3NlbGVjdG9yKDB4MzM5NjZCM0U2MDUwOTMxODVDQTBBMkE2NThDMTQ4MTkzOTc0RTgyMjJCOTc2REExMjREOTM2NjgwMDgxMDZDQSAweDI3RTY1M0M4MjE4MTI5RDhEQzZEQTM3MzAyMUE2M0Q0RkYxMDBFRDRBMjY3QzdCNkE3RDA3ODREMDA2NThCMDQpKShtdWxfc2VsZWN0b3IoMHgyNTU2OEYyRTlCNkU4OEQ2OTUxQjEzOTkwQjlCMzlBNDU5QzJCMjkyRTIwMkIzNUI5NkEwNzhGMTQ1OUNDODhDIDB4MDQyN0NDMzc2RUZGOENDN0U2REY1ODNBOTE5NDk5NDIxQzJCQTA2NUE0RUZFRDY1QzM5REE0MjkzMjhBNkQ4RSkpKGVtdWxfc2VsZWN0b3IoMHgzRTlEN0U4MjQ1NzVDQjE3REM1MTNFRDgwNUE0MEQ0ODJEMTk4QUM4NkIzQzhDMkE3OEUyNDc2MkNFNTIzMDgxIDB4MTcwOTMxMjg0NUQ1RjI2MkU4RkQ0RTIxNzgwNDA4QjM5MjYyQjlBQjg1QjNBMzlBQUExQzdCNEYwNzYwMDE5QykpKGVuZG9tdWxfc2NhbGFyX3NlbGVjdG9yKDB4MTJBQTA1MDdFQUFFRUUzOEQyNEQxNEMwNEM2RENCMjgzRjEyRTRDMDU0QjREMUQ3NURBRDRFQkU2NjMxM0M2RiAweDA1N0VCOUUwQ0I1MDQ0NTQ5Q0QzNDQ2MUFGMDdBMzNDQjIzMEI2QzZENDE4NjJEQkQ3NThDOTUzMjIzOUJGRDIpKSkpKGZ0X2V2YWwxIDB4MTk3RTBERDQxMTk2QThGOTZEN0MxOERBQjExRkQ3RjU1MjkwQkM5QURFMUJCNDIwMUZCNTFGQ0U0MTI2QUEyQykoYnVsbGV0cHJvb2YoKGxyKCgoMHgxQTNBRDFCRjU3MkIwMENCQUMyRTNBOTQ4MUYyRkNBNUJDQzBDQUQ2RUJEMzdGMUI4NEMzQkQxOEZDMDY4OEIzIDB4MzdFREM4RjIxQ0RGRkNBNDM0OTY4RUVBQzAyRDhDQ0YzNTU0QUY5RjQ2RjUwQUMwRDI4NjBFMzkzMjZCRjVDMikoMHgyM0MwQUQwMUY4Njg1QzJFNTI1MzVGNTRERDY5QjZDMEFGN0QyM0FBM0E0Qzc1RjFGNjcyQkI0N0ZEREEzNDE0IDB4MTAxRUE2ODhDN0M0RUFBRkQ1NDdFRUVEMjM0NDExRjZFMDE5OEY2MzQwMjk0NENEMjU1NTkzQzM2RUU4N0E2QikpKCgweDE0RTEzMUE3NTlGNDk3ODNDQkIwRDQ3MTVDOUQ2MjIwMDQ0OTQ4QUQ3MEEwQURGQ0VFNTY4NDg5NjI3NTJEMDcgMHgyREJBMkI5NzI2NEE3QTM4NkREMDJBOEI4RDY5RTEzRThGQjc2MThEMUJEQTFCRkQ1QkJENTBFMDJEQTUyOTc5KSgweDJFQjBCREEyNkRGRTM3NzZBMUU0OTMyNTI4MDBCNjZEOTQzRTlCQkRGMDBFMjY1RDcyN0QwMTFFM0Q3RUI0OTkgMHgxRkRGMDVCREE1M0YyNDZGMzREMzY4OTYyQjJDNEQxMjFCNzQ5MjMyMkIzNzM0QjA1RUZFNjc1RTdDRUM4NTQyKSkoKDB4M0REMTBEMDM2RTM2MTUyODM4NzhDQzMyMzczQzJEMEQ4MjhBQUIzQTVDNDU1ODUzNTE5MENCOTBCNkY3OEI4RCAweDAwMTc0NEVCNzhEQTU1NTZDNTJBMkIxNDg2NTE4NjYwMTVENEZCNzM1ODU0MjQwRDI0NkQxMjMxQ0Y5NEZBRjYpKDB4MkU5MDE5NTQxRjY4NjU2MUUyNUNEMUVFQ0Y1NzNDRTkxN0YwMTgyNzk5NkUxMTlGRUM3MTQ4MDA4MkU4NjZFNCAweDFBNzZDQjI2MjgwOTY5OUUyNEExOTRCNEJEODU1RUFDREQzQTg5M0E2M0NBMzZEMkIyNTk3NUFDMUY5MDRGMEYpKSgoMHgwRkRBNkIwNDdDMDEyMzcxMjkzQ0JDMEJDMkNERjQ1MjgwMjYzNjY1RURDMkZDQjZBMEM0Qjc4MDJCOTM5NkI5IDB4MTlBRkIwNjBGMThFQUM5Qjg3MDhGRDM4NzcwQkU5MkEwODJBNkExQjhBMjRENjE0N0JFMzA2QkVDMDc2RDQyRCkoMHgzREZCRjA1NTY2Q0ZCNUI4MDE5QjcxRjRCRDgzQjExMUU1ODE1QjczODhFQjcwNEI4NDg2QjlCQ0E2N0REQTAzIDB4MTAzQUI2MTMwOTYyNzVCMkIyRTVBNjlEMzI2Mjk5RUY2QjM5OTlDQkQ2MDQzMDMxRkM1MzgzNzFENUE3QjA2QykpKCgweDAzN0M1NjdEQTUwRTZCNDcwQjRGNTI2RTYxMDE1MTFCN0RDMTlENDdDNDhEMUEwMEFCNUNCQzZERjFBRUVCNzcgMHgzMkQwMTM0MUMzMUY5RjE1M0JGMjIxNTYyNjdBQUJCQjg1RDRDRkJDMEI5MkRDRjYzRDg2OThCMzBCNUQ0OUIzKSgweDM5RUM2OTc0RDAwNTlGMDhEQjA3MTZDMjQ1QzlGODYzOEJBNjU0NTYxQzdENkU1NzUwQzVGMzFFQ0Q1MEMxMTIgMHgxNDI3QzgxNjQ4RTk4QzI4RTFDNjQ1QUMyMjU2NkQ5RDM5OTkxMUQ0N0RDMTYzMDg4QTQ5NUFDNzMwQ0EzNEFFKSkoKDB4MjdDQjhFNTIwRTQxQzIzM0NFODI4MzdEREM3ODQ2RkEyQUY0MzhGRUE5NDA3N0FDMEM0RTY4RDdFRTYzNkI0RiAweDFGMzI1NUM1ODhCNjdCRDVGNDAzQzA4QjYzRkQyQTIyNTYwMzM1QTVCNDM3QjU1RDc1RjMyRDUzMTJDQzRFOUUpKDB4M0UzQzRBNTE2NDQ0NDdEOUMxNzhGOTQ0Q0FBNkIyQjYxQTgyMUI5NzlGRDAzRDk2NzFDMkU0RDg1NDNGNUExMCAweDEyNkI1REU5RjA5NUI5RkYyN0ExRkQ2RTc5N0FDNDZERjQ0NjI4MUQ2Q0NBOTdFMUZDRDM2ODg1MDhEQzYyOUUpKSgoMHgyMTU2QkFFMzk2RjUxMTBGRDlBRTAyNzc3MjA2MDdFNzNFQkVBNkZEOTA2NUZDNUExMUVCNTM3QUMzNTlEQzJGIDB4MDhCMTlBNUQwQzA4REJEN0Q1QTREMjQ1QjBGQzhEQTdGRDU1QTM4MDEyNzlFNUQwRkVFMDdBMTNBMTM4MDdEOCkoMHgzRUMwNTRDNkNEQjVBMDc0QTFBM0I3NTg4N0E4NzUwN0ZDRjgyNjg5MUY4RjEyMzcyRURDNDNGNDAzQ0JDQzgwIDB4MUYxMzk0MDBCRjBGN0RFQjhGQThEMUQ3NjNEMUQ3MDc2MDhCRUIyMTVDNEQ1MUNDRTc2NzFFOURDNjlEODJFQikpKCgweDE2RjU3NURFNjdEMzM1OTA3REUyOUNCMDc3Q0RGMDNCQ0EwM0RBNEJGNkNFMEMzRTlGM0M4MTBBRDFFNUJBOEQgMHgwOEIzOTNFQjg2QzUyNTg4MkY2NURBMUE0QTdERjk4MDNCOEFFRUNGODhCM0VBNURFNzBBMjVCOTc0RERBMjNEKSgweDMwNUNBMzkxQkRFNjk3MDEwRkU2NDA0RjIwRjkzMThBRUE1MjVDRDQwNkEwOUZFQ0VDQUQ5ODZGNjdBQzE0NkIgMHgyNzAwODNFNTJDQkNFRkYzM0QwNUJEMEE2OTdFMTQ4RjhFQUU5QzlBQTg5RUU4RTkxNkZDRURCNENDM0JBODlCKSkoKDB4MzBDRTJENDJFNDEzOEY5RjkzNjZDOTZENDc3NUIwREU0QTI2MUZCRkE5OEZEQUY5ODg5M0E0ODhBRkMyQ0QzRSAweDI4NTREOTUyMkUxREZEREE5RkRDODQ0MUY4MTY1NDY4MjUzQjVFNDZCNUNFNkFGMUJDOTBEQjk2NEFENzM0RDgpKDB4MzUzMzQxQTJDNERCMkExNjNGRTVFMEEzMDc4NzFEMDU4MTM5RERGMUREQTNFNDBFMTg0MDlEREQxMjYzNjQ4MiAweDMyMjlGQzQ3MjBCNDgzQzlDQUEwRTEzRTMwMTg0MEM3QzdBNEU5QUM3RjNBRTlBQzJCQzM2NTlEODMwNTZFRkYpKSgoMHgyMTdDNzI2Q0JDNEEyRTMwQzEyMkI1MkEwRUQzNDQ2MDVDMTkwRUYyMUU3NTM5QUI5NzQxMzA4RUE5NDIzNzNDIDB4MzlCQUYwMkVCNTQ5NEY3NEI5RUQ1M0U5NDk4MUQ1OUQwMTIxQ0JFNEJDMjU2QTI3Q0FFNTczMEQ3Mjc1OUNGMikoMHgxQzZGRjcyMjU0OEFBOUI2M0MwRTRBRDkxRTI2NzE1RkFGN0EzOUQ3NTA0QjA4NDhBRkU4QzM3QkFDQTYxQjkyIDB4MjIxMjYyMTMyMDg2OUJGRTE1N0MyNDlGQTk2OTcyOUM0RUMyNTJDOUU4NkUzOTgwRkY5MTg4QzAyNTc0Qjg4QSkpKCgweDMyM0Y5QTFERjY5RjY1MEIzQjY1NjU2QTk1QzYzNDcyM0VDM0NEQkI1QTU1QkJCQzk0OEM2RjJGQzNGMjFBNTAgMHgwOEFBQ0E0NTIxQ0EzQTQwOTVDMjZCRDNBQUQ3NDQ4OUFERDc5MjM4ODhGOENENzBGNDUyMDZGNzY4Q0I5MzhBKSgweDNDMEU5NTNENDNERTBCRjZGMjRGQ0VENkYwNzUyQjcwNkFDMkIxREMxMTI4OEY4MTVDN0ZCNDUxREFGNzI2NkMgMHgyMUQ5OTFCMzYzRjA0MUM1MTczODA0QkY2NDJGNjFGMTYyQzZCQ0RDNkI4M0Q4RjVEMkZEMkU1MjM1NzQ4MUU2KSkoKDB4MEE3NDE0RUFCNTI4MjZEQUIwOTQxRjVDNEFFNEExNUU4NUZCNjIwQTIwRjU2NzY5OTNDQUY0MjVCMEIzRkEzQiAweDFGRDMzQThGMDNDMUFGNDEwN0VEMjgyQzlDRkQwRkM0RjJCNzI2Mzk3RTMxRjM4QjJBNjcyRDVBQ0I2QzZBNzMpKDB4MTAwQTUzNTYzNzk5NTI4MzczQzEzNDY0RUI4MEU1OTlFOUI5QTNGNTk3QzlGRjJBNEQ0MzFEMkU1RjA5NDgyQyAweDNEQUU1RjA2N0M5OTI4NjYyRUZDNDIzNUUyQkUzMTg5MEQzRjA5NDU0ODgwRUU4NTFFOTQzMDkwQTU2MzM5NzYpKSgoMHgyM0EyRDAyOUI4MTI5Q0ZDQzkzRjkyQjZDNERGMkQ5MTJDQUZFNzkyODAxNjI3NDVBRDFGMTNDQjUyNUU2MTNBIDB4M0I5RUI4NkUyNjhEMUNEM0M1Q0I5RkQ5OTU1MjBCNUU3MTVGMTE2NUFDNUREQ0YwNkYyNTU2NDVFNzk5MUZGMikoMHgzRjU1NERGQ0FDMUM3NUIwMTk3N0E0MTNERkVCOUY1MjYwNTQwRDREMTlFRTZGNDQwMzUzMUM4ODlBQzlEOUU5IDB4MEYyMkJEMDY3M0NGRUMzMzM3RUY3ODhBREEzRDg0OTAwREREMTAwOUNFODlBQzFCNzkwMDcyRjQxRUFDNjgyMikpKCgweDI3NUM0QTIwNDY1NkZDNDQzRjkwMEY5QTI1OTQzMUM2NDgyRkIxOEJGQkE5MjIwMjkxRTAxRkIyRkZCM0Y1OTcgMHgwNEQ0QTgyRDBFMjc5RTMxQUQ3NTQ0ODBDNTcxNEUzMzM4QzE5NzdFMDczNUNDRkI0ODA1RkRGMDIwQTk3NEY1KSgweDMwRERFMDM2RUJEM0E1QkU2RTRBNzUzNkIxQTY1OUNGMTcwRDRFRjRDOEFGRTU3MjZDMDYxOUUyRDhGNzc0NUMgMHgxMkVCQURFMERFRkU4OEJEMENBMzg2RTY3MjI4NTM1RDFFQTE1RUJCRkRGMUEzMEY5Njc3QjNCQUNFNDNGMzlEKSkoKDB4M0VBRTYzMUVGRUYxMDk1NThDRjVEMEE4MEE3NDBGNjYwMENGREU4NENCMzdERkVGQjFFNTNDRkI5REVGNEZFNSAweDFBRjVFQ0Y0NTRDN0E2RDM0Nzk3RkFBNTRBNEVCMkMyRDZFNEFDRDQxNzAzNTVBMUYyODAzOTU5RDEwQUU1OTgpKDB4MEE5NTYzQUJEODhBMUEzRjIwRjBBOEQ0MUVFRUUyNzk2NDIwMTcwQUE1NzI4QUVCQzY5QkNCQUU0QzlDRTcxQiAweDJBREM0QTFERTUyMTkzRUJDMTU2N0VCQUE3NTY1NTU4NUE2RDIyMjUyNzhFNTdCOUEyNUVCQ0Y5NjVCNjRGRjApKSkpKHpfMSAweDFGMTA0M0QwQTVCQkYzNjM4REVCQjYyNkE0Q0FEQTY4NTFDQjQ1RENBRUZBMDkwNzVFNzIzOEY3OTc3NzFGMTUpKHpfMiAweDFDRTM0NzY4RjZDRjY4NzdCQUE1OTJCQ0E5QThGRDJCRTY0MUQ5ODFDRUVENDY1QzVBRTIwRTRBMDRDNUZENUYpKGRlbHRhKDB4MUZBQzZERTlGMTA3RTQ5MEU5NjMwODQ2QjJCMjg2MDg1RUQ2QTQyNDQzM0UxOThERTIzOTM2MjE5QUM4QTM3NiAweDI0OTMyMjQzMzBERUNENjEwODJGRkZDNTczQzczOTZGNzM2MzRFRDg3N0VCMTk0OTFFREQ3RjE0NDY0OTI0QUQpKShjaGFsbGVuZ2VfcG9seW5vbWlhbF9jb21taXRtZW50KDB4MENCRkJFMjk3NjU4NjFDMTNFMzU1RjJFQURGQkY0QkE4OEMwMjUyNUZFMDEyNjA4RUQ4Q0NGMTY4MjI3OTc2NCAweDBBRTU2NDkyREE3NEZDRDY5QTk0OUU0NDMxMzEzMUZERjk2QTA1MTFEM0Q1NDg3RDU3QUIwODIwQ0VCNzFCMzgpKSkpKSkp"
                            ]
                        },
                        "account_update_digest": "0x166BADD613EAC6EA1F7BDDE8E2A5155AA9A92DB2BA4112A57DCAC2935B00EA7B",
                        "calls": [
                            {
                                "elt": {
                                    "account_update": {
                                        "body": {
                                            "public_key": "B62qmZsubZXnjZr9XpTKJVtcyiDSDHN7o5tcctupQPgeLUZSMYnzqyH",
                                            "token_id": "xBxjFpJkbWpbGua7Lf36S1NLhffFoEChyP3pz6SYKnx7dFCTwg",
                                            "update": {
                                                "app_state": [
                                                    [
                                                        "Keep"
                                                    ],
                                                    [
                                                        "Keep"
                                                    ],
                                                    [
                                                        "Keep"
                                                    ],
                                                    [
                                                        "Keep"
                                                    ],
                                                    [
                                                        "Keep"
                                                    ],
                                                    [
                                                        "Keep"
                                                    ],
                                                    [
                                                        "Keep"
                                                    ],
                                                    [
                                                        "Keep"
                                                    ]
                                                ],
                                                "delegate": [
                                                    "Keep"
                                                ],
                                                "verification_key": [
                                                    "Keep"
                                                ],
                                                "permissions": [
                                                    "Keep"
                                                ],
                                                "zkapp_uri": [
                                                    "Keep"
                                                ],
                                                "token_symbol": [
                                                    "Keep"
                                                ],
                                                "timing": [
                                                    "Keep"
                                                ],
                                                "voting_for": [
                                                    "Keep"
                                                ]
                                            },
                                            "balance_change": {
                                                "magnitude": "1000000000",
                                                "sgn": [
                                                    "Neg"
                                                ]
                                            },
                                            "increment_nonce": false,
                                            "events": [],
                                            "actions": [],
                                            "call_data": "0x0000000000000000000000000000000000000000000000000000000000000000",
                                            "preconditions": {
                                                "network": {
                                                    "snarked_ledger_hash": [
                                                        "Ignore"
                                                    ],
                                                    "blockchain_length": [
                                                        "Ignore"
                                                    ],
                                                    "min_window_density": [
                                                        "Ignore"
                                                    ],
                                                    "total_currency": [
                                                        "Ignore"
                                                    ],
                                                    "global_slot_since_genesis": [
                                                        "Ignore"
                                                    ],
                                                    "staking_epoch_data": {
                                                        "ledger": {
                                                            "hash": [
                                                                "Ignore"
                                                            ],
                                                            "total_currency": [
                                                                "Ignore"
                                                            ]
                                                        },
                                                        "seed": [
                                                            "Ignore"
                                                        ],
                                                        "start_checkpoint": [
                                                            "Ignore"
                                                        ],
                                                        "lock_checkpoint": [
                                                            "Ignore"
                                                        ],
                                                        "epoch_length": [
                                                            "Ignore"
                                                        ]
                                                    },
                                                    "next_epoch_data": {
                                                        "ledger": {
                                                            "hash": [
                                                                "Ignore"
                                                            ],
                                                            "total_currency": [
                                                                "Ignore"
                                                            ]
                                                        },
                                                        "seed": [
                                                            "Ignore"
                                                        ],
                                                        "start_checkpoint": [
                                                            "Ignore"
                                                        ],
                                                        "lock_checkpoint": [
                                                            "Ignore"
                                                        ],
                                                        "epoch_length": [
                                                            "Ignore"
                                                        ]
                                                    }
                                                },
                                                "account": {
                                                    "balance": [
                                                        "Ignore"
                                                    ],
                                                    "nonce": [
                                                        "Ignore"
                                                    ],
                                                    "receipt_chain_hash": [
                                                        "Ignore"
                                                    ],
                                                    "delegate": [
                                                        "Ignore"
                                                    ],
                                                    "state": [
                                                        [
                                                            "Ignore"
                                                        ],
                                                        [
                                                            "Ignore"
                                                        ],
                                                        [
                                                            "Ignore"
                                                        ],
                                                        [
                                                            "Ignore"
                                                        ],
                                                        [
                                                            "Ignore"
                                                        ],
                                                        [
                                                            "Ignore"
                                                        ],
                                                        [
                                                            "Ignore"
                                                        ],
                                                        [
                                                            "Ignore"
                                                        ]
                                                    ],
                                                    "action_state": [
                                                        "Ignore"
                                                    ],
                                                    "proved_state": [
                                                        "Ignore"
                                                    ],
                                                    "is_new": [
                                                        "Ignore"
                                                    ]
                                                },
                                                "valid_while": [
                                                    "Ignore"
                                                ]
                                            },
                                            "use_full_commitment": true,
                                            "implicit_account_creation_fee": false,
                                            "may_use_token": [
                                                "Parents_own_token"
                                            ],
                                            "authorization_kind": [
                                                "Signature"
                                            ]
                                        },
                                        "authorization": [
                                            "Signature",
                                            "7mXWz3v3MtLCsSpircWaDjkP1cBK5kKC4v7Xz6rrUS8vqex4ENK83deE4Vetj4uZsnxXxeiSniddwkMuWQF5Bznu4ado5JrS"
                                        ]
                                    },
                                    "account_update_digest": "0x2373B862017F579E039F52903FFAAADB46DAFF092705375D2119E2647E5AF2D4",
                                    "calls": []
                                },
                                "stack_hash": "0x153BA2FAB998B07C181AA42A3F1BA8FF5853F3AF5B5F9FB0A865A65659CA0928"
                            },
                            {
                                "elt": {
                                    "account_update": {
                                        "body": {
                                            "public_key": "B62qjSHAcwTouw5pxYECuJSFtmG6xup3DeK6f5BWW3BBhvEumW6daEm",
                                            "token_id": "xBxjFpJkbWpbGua7Lf36S1NLhffFoEChyP3pz6SYKnx7dFCTwg",
                                            "update": {
                                                "app_state": [
                                                    [
                                                        "Keep"
                                                    ],
                                                    [
                                                        "Keep"
                                                    ],
                                                    [
                                                        "Keep"
                                                    ],
                                                    [
                                                        "Keep"
                                                    ],
                                                    [
                                                        "Keep"
                                                    ],
                                                    [
                                                        "Keep"
                                                    ],
                                                    [
                                                        "Keep"
                                                    ],
                                                    [
                                                        "Keep"
                                                    ]
                                                ],
                                                "delegate": [
                                                    "Keep"
                                                ],
                                                "verification_key": [
                                                    "Keep"
                                                ],
                                                "permissions": [
                                                    "Keep"
                                                ],
                                                "zkapp_uri": [
                                                    "Keep"
                                                ],
                                                "token_symbol": [
                                                    "Keep"
                                                ],
                                                "timing": [
                                                    "Keep"
                                                ],
                                                "voting_for": [
                                                    "Keep"
                                                ]
                                            },
                                            "balance_change": {
                                                "magnitude": "1000000000",
                                                "sgn": [
                                                    "Pos"
                                                ]
                                            },
                                            "increment_nonce": false,
                                            "events": [],
                                            "actions": [],
                                            "call_data": "0x0000000000000000000000000000000000000000000000000000000000000000",
                                            "preconditions": {
                                                "network": {
                                                    "snarked_ledger_hash": [
                                                        "Ignore"
                                                    ],
                                                    "blockchain_length": [
                                                        "Ignore"
                                                    ],
                                                    "min_window_density": [
                                                        "Ignore"
                                                    ],
                                                    "total_currency": [
                                                        "Ignore"
                                                    ],
                                                    "global_slot_since_genesis": [
                                                        "Ignore"
                                                    ],
                                                    "staking_epoch_data": {
                                                        "ledger": {
                                                            "hash": [
                                                                "Ignore"
                                                            ],
                                                            "total_currency": [
                                                                "Ignore"
                                                            ]
                                                        },
                                                        "seed": [
                                                            "Ignore"
                                                        ],
                                                        "start_checkpoint": [
                                                            "Ignore"
                                                        ],
                                                        "lock_checkpoint": [
                                                            "Ignore"
                                                        ],
                                                        "epoch_length": [
                                                            "Ignore"
                                                        ]
                                                    },
                                                    "next_epoch_data": {
                                                        "ledger": {
                                                            "hash": [
                                                                "Ignore"
                                                            ],
                                                            "total_currency": [
                                                                "Ignore"
                                                            ]
                                                        },
                                                        "seed": [
                                                            "Ignore"
                                                        ],
                                                        "start_checkpoint": [
                                                            "Ignore"
                                                        ],
                                                        "lock_checkpoint": [
                                                            "Ignore"
                                                        ],
                                                        "epoch_length": [
                                                            "Ignore"
                                                        ]
                                                    }
                                                },
                                                "account": {
                                                    "balance": [
                                                        "Ignore"
                                                    ],
                                                    "nonce": [
                                                        "Ignore"
                                                    ],
                                                    "receipt_chain_hash": [
                                                        "Ignore"
                                                    ],
                                                    "delegate": [
                                                        "Ignore"
                                                    ],
                                                    "state": [
                                                        [
                                                            "Ignore"
                                                        ],
                                                        [
                                                            "Ignore"
                                                        ],
                                                        [
                                                            "Ignore"
                                                        ],
                                                        [
                                                            "Ignore"
                                                        ],
                                                        [
                                                            "Ignore"
                                                        ],
                                                        [
                                                            "Ignore"
                                                        ],
                                                        [
                                                            "Ignore"
                                                        ],
                                                        [
                                                            "Ignore"
                                                        ]
                                                    ],
                                                    "action_state": [
                                                        "Ignore"
                                                    ],
                                                    "proved_state": [
                                                        "Ignore"
                                                    ],
                                                    "is_new": [
                                                        "Ignore"
                                                    ]
                                                },
                                                "valid_while": [
                                                    "Ignore"
                                                ]
                                            },
                                            "use_full_commitment": false,
                                            "implicit_account_creation_fee": false,
                                            "may_use_token": [
                                                "Parents_own_token"
                                            ],
                                            "authorization_kind": [
                                                "None_given"
                                            ]
                                        },
                                        "authorization": [
                                            "None_given"
                                        ]
                                    },
                                    "account_update_digest": "0x33CAF71259C5274030403D7EE078157448831DBE88999890A2BB28CBA83F1A17",
                                    "calls": []
                                },
                                "stack_hash": "0x05DA4F13B136476762228BE6DB195F8C2738F40DDC51365D392E693426489BA0"
                            }
                        ]
                    },
                    "stack_hash": "0x382F7269A04CFEDECF05F1B5DEC8BC6B9C8DA6519C5057A797F63B89FC2FF9D6"
                }
            ],
            "memo": "E4YnpiqYQCCH2ZafMKGydUjDtdkzUsEDuVQ9wAXR8Pq3S3iGdeg7v"
        }
    ],
    "status": [
        "Applied"
    ]
}
```
