use mina_indexer::{
    block::parser::BlockParser,
    ledger::{diff::LedgerDiff, Ledger},
};
use std::path::PathBuf;

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

    println!("=== Initial ===");
    println!("{:?}", ledger);

    let ledger = ledger.apply_diff(&diff)?;
    let expected = Ledger::from(vec![
        (
            "B62qiburnzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzmp7r7UN6X",
            0, // TODO: probably this address should be credited the account creation fee
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
            1155800000000,
            None,
            None,
        ),
    ])?;

    println!("=== Diff ===");
    println!("{diff:?}");

    println!("=== Final ===");
    println!("{ledger:?}");

    for (pk, account) in ledger.accounts.iter() {
        if Some(account) != expected.accounts.get(pk) {
            println!(
                "ledger != expect\n{account}\n{}",
                expected.accounts.get(pk).unwrap()
            );
        }
    }
    assert_eq!(ledger, expected);
    Ok(())
}
