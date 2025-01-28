use mina_indexer::{
    base::public_key::PublicKey,
    block::parser::BlockParser,
    ledger::{diff::LedgerDiff, Ledger},
};
use std::{collections::HashSet, path::PathBuf, str::FromStr};

#[tokio::test]
async fn account_diffs() -> anyhow::Result<()> {
    let log_dir = PathBuf::from("./tests/data/sequential_blocks");
    let mut block_parser = BlockParser::new_testing(&log_dir)?;

    // mainnet-105490-3NKxEA9gztvEGxL4uk4eTncZAxuRmMsB8n81UkeAMevUjMbLHmkC.json
    let (block, _) = block_parser
        .get_precomputed_block("3NKxEA9gztvEGxL4uk4eTncZAxuRmMsB8n81UkeAMevUjMbLHmkC")
        .await?;
    let diff = LedgerDiff::from_precomputed(&block);
    let ledger = Ledger::from(vec![
        (
            "B62qrRvo5wngd5WA1dgXkQpCdQMRDndusmjfWXWT1LgsSFFdBS9RCsV",
            1000000000000,
            None,
            None,
        ),
        (
            "B62qrdhG66vK71Jbdz6Xs7cnDxQ8f6jZUFvefkp3pje4EejYUTvotGP",
            1000000000000,
            None,
            None,
        ),
        (
            "B62qqLa7eh6FNPH4hCw2oB7qhA5HuKtMyqnNRnD7KyGR3McaATPjahL",
            1000000000000,
            None,
            None,
        ),
        (
            "B62qjYanmV7y9njVeH5UHkz3GYBm7xKir1rAnoY4KsEYUGLMiU45FSM",
            1000000000000,
            None,
            None,
        ),
        (
            "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy",
            1000000000000,
            None,
            None,
        ),
        (
            "B62qq66ZuaVGxVvNwR752jPoZfN4uyZWrKkLeBS8FxdG9S76dhscRLy",
            1000000000000,
            None,
            None,
        ),
        (
            "B62qrusueb8gq1RbZWyZG9EN1eCKjbByTQ39fgiGigkvg7nJR3VdGwX",
            1000000000000,
            None,
            None,
        ),
        (
            "B62qqhURJQo3CvWC3WFo9LhUhtcaJWLBcJsaA3DXaU2GH5KgXujZiwB",
            1000000000000,
            None,
            None,
        ),
    ])?;

    let only_new_pk =
        PublicKey::from_str("B62qq66ZuaVGxVvNwR752jPoZfN4uyZWrKkLeBS8FxdG9S76dhscRLy").unwrap();
    let new_pks = diff.new_pk_balances.keys().collect::<HashSet<&PublicKey>>();
    assert_eq!(new_pks, HashSet::from([&only_new_pk]));

    println!("=== Initial ===");
    println!("{:?}", ledger);

    let ledger = ledger.apply_diff(&diff)?;
    let expected = Ledger::from(vec![
        (
            "B62qrusueb8gq1RbZWyZG9EN1eCKjbByTQ39fgiGigkvg7nJR3VdGwX",
            1000000000000,
            None,
            None,
        ),
        (
            "B62qrRvo5wngd5WA1dgXkQpCdQMRDndusmjfWXWT1LgsSFFdBS9RCsV",
            843190000000,
            Some(42428),
            None,
        ),
        (
            "B62qrdhG66vK71Jbdz6Xs7cnDxQ8f6jZUFvefkp3pje4EejYUTvotGP",
            2439634213000,
            Some(7297),
            None,
        ),
        (
            "B62qqhURJQo3CvWC3WFo9LhUhtcaJWLBcJsaA3DXaU2GH5KgXujZiwB",
            1000000000000,
            None,
            None,
        ),
        (
            "B62qqLa7eh6FNPH4hCw2oB7qhA5HuKtMyqnNRnD7KyGR3McaATPjahL",
            1000377787000,
            None,
            None,
        ),
        (
            "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy",
            999997998000,
            Some(146494),
            None,
        ),
        (
            "B62qjYanmV7y9njVeH5UHkz3GYBm7xKir1rAnoY4KsEYUGLMiU45FSM",
            1000000002000,
            None,
            None,
        ),
        (
            "B62qq66ZuaVGxVvNwR752jPoZfN4uyZWrKkLeBS8FxdG9S76dhscRLy",
            1156800000000,
            None,
            None,
        ),
    ])?;

    println!("=== Diff ===");
    println!("{diff:#?}");

    println!("=== Final ===");
    println!("{ledger:?}");

    for (token, token_ledger) in ledger.tokens.iter() {
        for (pk, pk_ledger) in token_ledger.accounts.iter() {
            assert_eq!(pk_ledger, expected.get_account(pk, token).unwrap());
        }
    }

    Ok(())
}
