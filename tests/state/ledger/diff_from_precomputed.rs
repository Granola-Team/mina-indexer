use std::{collections::HashMap, path::PathBuf};

use mina_indexer::{
    block::parser::BlockParser,
    state::ledger::{
        diff::{
            account::{AccountDiff, DelegationDiff, PaymentDiff, UpdateType},
            LedgerDiff,
        },
        public_key::PublicKey,
    },
};

#[tokio::test]
async fn account_diffs() {
    let log_dir = PathBuf::from("./tests/data/beautified_sequential_blocks");
    let mut block_parser = BlockParser::new(&log_dir).unwrap();

    // mainnet-105490-3NKxEA9gztvEGxL4uk4eTncZAxuRmMsB8n81UkeAMevUjMbLHmkC.json
    let block = block_parser
        .get_precomputed_block("3NKxEA9gztvEGxL4uk4eTncZAxuRmMsB8n81UkeAMevUjMbLHmkC")
        .await
        .unwrap();
    let diff = LedgerDiff::from_precomputed_block(&block);

    let mut ledger: HashMap<PublicKey, i64> = HashMap::from([
        (
            PublicKey::from_address("B62qrRvo5wngd5WA1dgXkQpCdQMRDndusmjfWXWT1LgsSFFdBS9RCsV")
                .unwrap(),
            1000000000000,
        ),
        (
            PublicKey::from_address("B62qrdhG66vK71Jbdz6Xs7cnDxQ8f6jZUFvefkp3pje4EejYUTvotGP")
                .unwrap(),
            1000000000000,
        ),
        (
            PublicKey::from_address("B62qqLa7eh6FNPH4hCw2oB7qhA5HuKtMyqnNRnD7KyGR3McaATPjahL")
                .unwrap(),
            1000000000000,
        ),
        (
            PublicKey::from_address("B62qjYanmV7y9njVeH5UHkz3GYBm7xKir1rAnoY4KsEYUGLMiU45FSM")
                .unwrap(),
            1000000000000,
        ),
        (
            PublicKey::from_address("B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy")
                .unwrap(),
            1000000000000,
        ),
        (
            PublicKey::from_address("B62qq66ZuaVGxVvNwR752jPoZfN4uyZWrKkLeBS8FxdG9S76dhscRLy")
                .unwrap(),
            1000000000000,
        ),
    ]);

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
                println!("amount:      {amount}");
                println!("update_type: {update_type:?}");

                match update_type {
                    UpdateType::Deduction => {
                        if let Some(balance) = ledger.get_mut(&public_key) {
                            if amount as i64 > *balance {
                                println!("deduction amount exceeded balance");
                                panic!();
                            }
                            *balance -= amount as i64;
                        }
                    }
                    UpdateType::Deposit => {
                        if let Some(balance) = ledger.get_mut(&public_key) {
                            *balance += amount as i64;
                        } else {
                            ledger.insert(public_key, amount as i64);
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
        }
    }

    let delta: HashMap<PublicKey, i64> = HashMap::from([
        (
            PublicKey::from_address("B62qrRvo5wngd5WA1dgXkQpCdQMRDndusmjfWXWT1LgsSFFdBS9RCsV")
                .unwrap(),
            -156810000000,
        ),
        (
            PublicKey::from_address("B62qrdhG66vK71Jbdz6Xs7cnDxQ8f6jZUFvefkp3pje4EejYUTvotGP")
                .unwrap(),
            1439634213000,
        ),
        (
            PublicKey::from_address("B62qqLa7eh6FNPH4hCw2oB7qhA5HuKtMyqnNRnD7KyGR3McaATPjahL")
                .unwrap(),
            377787000,
        ),
        (
            PublicKey::from_address("B62qjYanmV7y9njVeH5UHkz3GYBm7xKir1rAnoY4KsEYUGLMiU45FSM")
                .unwrap(),
            2000,
        ),
        (
            PublicKey::from_address("B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy")
                .unwrap(),
            -2002000,
        ),
        (
            PublicKey::from_address("B62qq66ZuaVGxVvNwR752jPoZfN4uyZWrKkLeBS8FxdG9S76dhscRLy")
                .unwrap(),
            156800000000,
        ),
    ]);

    for pk in ledger.keys() {
        let diff = ledger.get(&pk).unwrap() - initial_ledger.get(&pk).unwrap();
        assert_eq!(delta.get(&pk).unwrap(), &diff);
    }
}
