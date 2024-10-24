use mina_indexer::{
    block::parser::BlockParser,
    ledger::{
        diff::{
            account::{
                AccountDiff, CoinbaseDiff, DelegationDiff, FailedTransactionNonceDiff, PaymentDiff,
                UpdateType,
            },
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
    let (block, _) = block_parser
        .get_precomputed_block("3NKxEA9gztvEGxL4uk4eTncZAxuRmMsB8n81UkeAMevUjMbLHmkC")
        .await
        .unwrap();
    let diff = LedgerDiff::from_precomputed(&block);
    let mut ledger: HashMap<PublicKey, (i64, u32)> = HashMap::from(
        [
            (
                "B62qrRvo5wngd5WA1dgXkQpCdQMRDndusmjfWXWT1LgsSFFdBS9RCsV",
                (1000000000000, 0),
            ),
            (
                "B62qrdhG66vK71Jbdz6Xs7cnDxQ8f6jZUFvefkp3pje4EejYUTvotGP",
                (1000000000000, 0),
            ),
            (
                "B62qqLa7eh6FNPH4hCw2oB7qhA5HuKtMyqnNRnD7KyGR3McaATPjahL",
                (1000000000000, 0),
            ),
            (
                "B62qjYanmV7y9njVeH5UHkz3GYBm7xKir1rAnoY4KsEYUGLMiU45FSM",
                (1000000000000, 0),
            ),
            (
                "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy",
                (1000000000000, 0),
            ),
            (
                "B62qq66ZuaVGxVvNwR752jPoZfN4uyZWrKkLeBS8FxdG9S76dhscRLy",
                (1000000000000, 0),
            ),
        ]
        .map(|(pk, amt)| (PublicKey::new(pk), amt)),
    );
    let initial_ledger = ledger.clone();

    println!("=== Account diffs ===");
    for diff in diff.account_diffs.iter().flatten() {
        match diff {
            AccountDiff::Payment(PaymentDiff {
                public_key,
                amount,
                update_type,
            })
            | AccountDiff::FeeTransfer(PaymentDiff {
                public_key,
                amount,
                update_type,
            })
            | AccountDiff::FeeTransferViaCoinbase(PaymentDiff {
                public_key,
                amount,
                update_type,
            }) => {
                println!("\n* Payment");
                println!("public_key:  {public_key}");
                println!("amount:      {}", amount.0);
                println!("update_type: {update_type:?}");

                match update_type {
                    UpdateType::Debit(new_nonce) => {
                        if let Some((balance, nonce)) = ledger.get_mut(public_key) {
                            if amount.0 as i64 > *balance {
                                println!("Debit amount exceeded balance");
                                panic!();
                            }
                            *balance -= amount.0 as i64;

                            if let Some(new_nonce) = new_nonce {
                                *nonce = new_nonce.0;
                            }
                        }
                    }
                    UpdateType::Credit => {
                        if let Some((balance, _)) = ledger.get_mut(public_key) {
                            *balance += amount.0 as i64;
                        } else {
                            ledger.insert(public_key.clone(), (amount.0 as i64, 0));
                        }
                    }
                }
            }
            AccountDiff::Delegation(DelegationDiff {
                delegate,
                delegator,
                nonce,
            }) => {
                println!("\n* Delegation");
                println!("delegate:  {delegate}");
                println!("delegator: {delegator}");
                println!("nonce:     {nonce}");
            }
            AccountDiff::Coinbase(CoinbaseDiff { public_key, amount }) => {
                println!("\n* Coinbase");
                println!("public_key: {public_key}");
                println!("amount:     {}", amount.0);
                if let Some((balance, _)) = ledger.get_mut(public_key) {
                    *balance += amount.0 as i64;
                } else {
                    ledger.insert(public_key.clone(), (amount.0 as i64, 0));
                }
            }
            AccountDiff::FailedTransactionNonce(FailedTransactionNonceDiff {
                public_key,
                nonce: new_nonce,
            }) => {
                println!("\n* Failed transaction");
                println!("public_key: {public_key}");
                println!("nonce:      {new_nonce}");
                if let Some((_, nonce)) = ledger.get_mut(public_key) {
                    *nonce += new_nonce.0;
                }
            }
            AccountDiff::Zkapp(_zkapp) => todo!(),
        }
    }

    let delta: HashMap<PublicKey, (i64, u32)> = HashMap::from(
        [
            (
                "B62qrRvo5wngd5WA1dgXkQpCdQMRDndusmjfWXWT1LgsSFFdBS9RCsV",
                (-156810000000, 42428),
            ),
            (
                "B62qrdhG66vK71Jbdz6Xs7cnDxQ8f6jZUFvefkp3pje4EejYUTvotGP",
                (1439634213000, 7297),
            ),
            (
                "B62qqLa7eh6FNPH4hCw2oB7qhA5HuKtMyqnNRnD7KyGR3McaATPjahL",
                (377787000, 0),
            ),
            (
                "B62qjYanmV7y9njVeH5UHkz3GYBm7xKir1rAnoY4KsEYUGLMiU45FSM",
                (2000, 0),
            ),
            (
                "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy",
                (-2002000, 146494),
            ),
            (
                "B62qq66ZuaVGxVvNwR752jPoZfN4uyZWrKkLeBS8FxdG9S76dhscRLy",
                (156800000000, 0),
            ),
        ]
        .map(|(pk, amt)| (PublicKey::new(pk), amt)),
    );

    for pk in ledger.keys() {
        let balance_diff = ledger.get(pk).unwrap().0 - initial_ledger.get(pk).unwrap().0;
        let nonce_diff = ledger.get(pk).unwrap().1 - initial_ledger.get(pk).unwrap().1;

        if delta.get(pk).unwrap() != &(balance_diff, nonce_diff) {
            println!("Incorrect delta for {}", pk.to_address());
            println!("Final:   {:?}", ledger.get(pk).unwrap());
            println!("Initial: {:?}", initial_ledger.get(pk).unwrap());
        }

        assert_eq!(delta.get(pk).unwrap(), &(balance_diff, nonce_diff));
    }
}
