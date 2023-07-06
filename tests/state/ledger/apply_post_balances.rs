use std::path::PathBuf;

use mina_indexer::{block::parser::BlockParser, state::ledger::Ledger};

#[tokio::test]
async fn post_balances() {
    let log_dir = PathBuf::from("./tests/data/sequential_blocks");
    let mut block_parser = BlockParser::new_testing(&log_dir).unwrap();

    // mainnet-105490-3NKxEA9gztvEGxL4uk4eTncZAxuRmMsB8n81UkeAMevUjMbLHmkC.json
    let block = block_parser
        .get_precomputed_block("3NKxEA9gztvEGxL4uk4eTncZAxuRmMsB8n81UkeAMevUjMbLHmkC")
        .await
        .unwrap();

    let mut ledger = Ledger::from(vec![
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
    ])
    .unwrap();

    println!("=== Initial ===");
    println!("{:?}", ledger);

    ledger.apply_post_balances(&block);

    let expected = Ledger::from(vec![
        (
            "B62qrusueb8gq1RbZWyZG9EN1eCKjbByTQ39fgiGigkvg7nJR3VdGwX",
            1000000000000,
            None,
            None,
        ),
        (
            "B62qrRvo5wngd5WA1dgXkQpCdQMRDndusmjfWXWT1LgsSFFdBS9RCsV",
            69533839718740403,
            Some(42428),
            None,
        ),
        (
            "B62qrdhG66vK71Jbdz6Xs7cnDxQ8f6jZUFvefkp3pje4EejYUTvotGP",
            66859024736773,
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
            111313525000,
            None,
            None,
        ),
        (
            "B62qre3erTHfzQckNuibViWQGyyKwZseztqrjPZBv6SQF384Rg6ESAy",
            947717525111,
            Some(146494),
            None,
        ),
        (
            "B62qjYanmV7y9njVeH5UHkz3GYBm7xKir1rAnoY4KsEYUGLMiU45FSM",
            860262858,
            None,
            None,
        ),
        (
            "B62qq66ZuaVGxVvNwR752jPoZfN4uyZWrKkLeBS8FxdG9S76dhscRLy",
            155800000000,
            None,
            None,
        ),
    ])
    .unwrap();

    println!("=== Final ===");
    println!("{ledger:?}");

    assert_eq!(ledger, expected);
}
