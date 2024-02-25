use mina_indexer::{
    block::parser::BlockParser,
    ledger::{
        diff::{
            account::{AccountDiff, CoinbaseDiff, DelegationDiff, PaymentDiff, UpdateType},
            LedgerDiff,
        },
        public_key::PublicKey,
    },
};
use std::{collections::HashMap, path::PathBuf};

#[tokio::test]
async fn account_diffs() {
    let log_dir = PathBuf::from("./tests/data/sequential_blocks");
    let mut block_parser = BlockParser::new_testing(&log_dir).unwrap();

    // mainnet-105490-3NKxEA9gztvEGxL4uk4eTncZAxuRmMsB8n81UkeAMevUjMbLHmkC.json
    let block = block_parser
        .get_precomputed_block("3NKxEA9gztvEGxL4uk4eTncZAxuRmMsB8n81UkeAMevUjMbLHmkC")
        .await
        .unwrap();
    let diff = LedgerDiff::from_precomputed(&block);

    let mut ledger: HashMap<PublicKey, i64> = HashMap::from(
        [
            (
                "B62qrRvo5wngd5WA1dgXkQpCdQMRDndusmjfWXWT1LgsSFFdBS9RCsV",
                1000000000000,
            ),
            (
                "B62qrdhG66vK71Jbdz6Xs7cnDxQ8f6jZUFvefkp3pje4EejYUTvotGP",
                1000000000000,
            ),
            (
                "B62qqLa7eh6FNPH4hCw2oB7qhA5HuKtMyqnNRnD7KyGR3McaATPjahL",
                1000000000000,
            ),
            (
                "B62qjYanmV7y9njVeH5UHkz3GYBm7xKir1rAnoY4KsEYUGLMiU45FSM",
                1000000000000,
            ),
            (
                "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy",
                1000000000000,
            ),
            (
                "B62qq66ZuaVGxVvNwR752jPoZfN4uyZWrKkLeBS8FxdG9S76dhscRLy",
                1000000000000,
            ),
        ]
        .map(|(pk, amt)| (PublicKey::new(pk), amt)),
    );

    let initial_ledger = ledger.clone();

    println!("=== Account diffs ===");
    for x in diff.account_diffs {
        match x {
            AccountDiff::Payment(PaymentDiff {
                public_key,
                amount,
                update_type,
            }) => {
                println!("\n* Payment");
                println!("public_key:  {public_key:?}");
                println!("amount:      {}", amount.0);
                println!("update_type: {update_type:?}");

                match update_type {
                    UpdateType::Deduction => {
                        if let Some(balance) = ledger.get_mut(&public_key) {
                            if amount.0 as i64 > *balance {
                                println!("deduction amount exceeded balance");
                                panic!();
                            }
                            *balance -= amount.0 as i64;
                        }
                    }
                    UpdateType::Deposit => {
                        if let Some(balance) = ledger.get_mut(&public_key) {
                            *balance += amount.0 as i64;
                        } else {
                            ledger.insert(public_key, amount.0 as i64);
                        }
                    }
                }
            }
            AccountDiff::Delegation(DelegationDiff {
                delegate,
                delegator,
            }) => {
                println!("\n* Delegation");
                println!("delegate:  {delegate:?}");
                println!("delegator: {delegator:?}");
            }
            AccountDiff::Coinbase(CoinbaseDiff { public_key, amount }) => {
                println!("\n* Coinbase");
                println!("public_key: {public_key:?}");
                println!("amount:     {}", amount.0);

                if let Some(balance) = ledger.get_mut(&public_key) {
                    *balance += amount.0 as i64;
                } else {
                    ledger.insert(public_key, amount.0 as i64);
                }
            }
        }
    }

    let delta: HashMap<PublicKey, i64> = HashMap::from(
        [
            (
                "B62qrRvo5wngd5WA1dgXkQpCdQMRDndusmjfWXWT1LgsSFFdBS9RCsV",
                -156810000000,
            ),
            (
                "B62qrdhG66vK71Jbdz6Xs7cnDxQ8f6jZUFvefkp3pje4EejYUTvotGP",
                1439634213000,
            ),
            (
                "B62qqLa7eh6FNPH4hCw2oB7qhA5HuKtMyqnNRnD7KyGR3McaATPjahL",
                377787000,
            ),
            (
                "B62qjYanmV7y9njVeH5UHkz3GYBm7xKir1rAnoY4KsEYUGLMiU45FSM",
                2000,
            ),
            (
                "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy",
                -2002000,
            ),
            (
                "B62qq66ZuaVGxVvNwR752jPoZfN4uyZWrKkLeBS8FxdG9S76dhscRLy",
                156800000000,
            ),
        ]
        .map(|(pk, amt)| (PublicKey::new(pk), amt)),
    );

    for pk in ledger.keys() {
        let diff = ledger.get(pk).unwrap() - initial_ledger.get(pk).unwrap();

        if delta.get(pk).unwrap() != &diff {
            println!("Incorrect delta for {}", pk.to_address());
            println!("Final:   {}", ledger.get(pk).unwrap());
            println!("Initial: {}", initial_ledger.get(pk).unwrap());
        }

        assert_eq!(delta.get(pk).unwrap(), &diff);
    }
}
