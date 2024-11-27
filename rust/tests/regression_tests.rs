use mina_indexer::constants::POSTGRES_CONNECTION_STRING;
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    io::{BufRead, BufReader},
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use tokio_postgres::NoTls;

#[derive(Debug, Deserialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Account {
    public_key: String,
    balance: u64,
}

#[derive(Debug, Deserialize, Hash, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct Block {
    state_hash: String,
    block_height: u64,
    canonical: bool,
}

// #[tokio::test]
// async fn test_first_100_blocks() {
//     run_test_process(
//         env!("CARGO_BIN_EXE_ingestion"), // Binary path
//         &[("BLOCKS_DIR", "./tests/data/5000_mainnet_blocks"), ("PUBLISH_RATE_PER_SECOND", "20")],
//         &[],
//         Duration::from_secs(20),
//     );

//     truncate_table("blocks_log_0", 100).await;
//     test_blocks_first_100().await;

//     // restart ingestion from block 52
//     run_test_process(
//         env!("CARGO_BIN_EXE_ingestion"), // Binary path
//         &[("BLOCKS_DIR", "./tests/data/5000_mainnet_blocks"), ("PUBLISH_RATE_PER_SECOND", "20")],
//         &["52", "3NLY4ci13GgX1GGkUrfTKr1Zt69U6Xj546d15nU9o1WrfHVnbosd"],
//         Duration::from_secs(10),
//     );

//     truncate_table("blocks_log_0", 100).await;
//     test_blocks_first_100().await;
// }

#[tokio::test]
async fn test_blockchain_ledger() {
    run_test_process(
        env!("CARGO_BIN_EXE_ingestion"), // Binary path
        &[("BLOCKS_DIR", "./tests/data/5000_mainnet_blocks"), ("PUBLISH_RATE_PER_SECOND", "20")],
        &[],
        Duration::from_secs(12 * 60),
    );

    truncate_table("blockchain_ledger", 5000).await;
    test_ledger_ingested_up_to(5000).await;
    test_blockchain_ledger_accounting_per_block().await;
    test_account_balances().await;

    // restart ingestion from height 4501
    run_test_process(
        env!("CARGO_BIN_EXE_ingestion"), // Binary path
        &[("BLOCKS_DIR", "./tests/data/5000_mainnet_blocks"), ("PUBLISH_RATE_PER_SECOND", "20")],
        &["4501", "3NLRAYzM1oPghMUJWaQqMrdpaAwj4FpQ8BUw6GQ8e9Q4zW3K864F"],
        Duration::from_secs(2 * 60),
    );

    truncate_table("blockchain_ledger", 5000).await;
    test_ledger_ingested_up_to(5000).await;
    test_blockchain_ledger_accounting_per_block().await;
    test_account_balances().await;
}

async fn truncate_table(table: &str, height: u64) {
    if let Ok((client, connection)) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls).await {
        let handle = tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        let query = format!("DELETE FROM {} WHERE height > {};", table, height);

        // Execute the query using the SQL client from the actor
        if let Err(e) = client.execute(query.as_str(), &[]).await.map_err(|_| "Unable to trim the blockchain ledger") {
            eprintln!("{}", e);
        }

        drop(handle);
    } else {
        panic!("Unable to open connection to database");
    }
}

async fn test_ledger_ingested_up_to(x: u64) {
    if let Ok((client, connection)) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls).await {
        let handle = tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        let query = r#"
            SELECT
           	CAST(MAX(height) AS BIGINT) AS max_height
                    FROM
           	blockchain_ledger;
        "#;

        // Execute the query using the SQL client from the actor
        if let Ok(row) = client.query_one(query, &[]).await.map_err(|_| "Unable to get max height of blockchain ledger") {
            let max_height: i64 = row.get("max_height");
            assert_eq!(max_height, x as i64, "Expected the ledger to have been ingested up to height ");
        } else {
            panic!("Could not execute query")
        }
        drop(handle);
    } else {
        panic!("Unable to open connection to database");
    }
}

async fn test_blockchain_ledger_accounting_per_block() {
    if let Ok((client, connection)) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls).await {
        let handle = tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        let query = r#"
            SELECT CAST(SUM(balance_delta_sum) AS BIGINT) AS total_balance
            FROM (
                SELECT SUM(balance_delta) AS balance_delta_sum
                FROM blockchain_ledger
                GROUP BY height, state_hash
            ) AS grouped_sums;
        "#;

        // Execute the query using the SQL client from the actor
        if let Ok(row) = client.query_one(query, &[]).await.map_err(|_| "Unable to fetch total balance delta sum") {
            let total_balance: i64 = row.get("total_balance");
            assert_eq!(total_balance, 0, "Expected accounting to balance within each block");
        } else {
            panic!("Could not execute query")
        }
        drop(handle);
    } else {
        panic!("Unable to open connection to database");
    }
}

async fn test_blocks_first_100() {
    let file_content = std::fs::read_to_string(Path::new("./tests/data/canonicity_of_first_100_blocks.json")).expect("Failed to read JSON file from disk");

    let blocks: Vec<Block> = sonic_rs::from_str(&file_content).unwrap();

    // Create a HashMap with composite keys (height, state_hash)
    let file_blocks_map: HashMap<(u64, String), bool> = blocks
        .into_iter()
        .map(|block| ((block.block_height, block.state_hash.clone()), block.canonical))
        .collect();

    let (client, connection) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls)
        .await
        .expect("Expected to open connection");

    let handle = tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let query = r#"
        SELECT height, state_hash, canonical
        FROM blocks;
    "#;

    let rows = client.query(query, &[]).await.expect("Failed to execute query");

    // Create a HashMap from database rows with composite keys
    let db_blocks_map: HashMap<(u64, String), bool> = rows
        .into_iter()
        .map(|row| {
            (
                (row.get::<_, i64>("height") as u64, row.get::<_, String>("state_hash")),
                row.get::<_, bool>("canonical"),
            )
        })
        .collect();

    // Ensure the sizes match
    // Calculate the symmetric difference
    let file_keys: HashSet<_> = file_blocks_map.keys().collect();
    let db_keys: HashSet<_> = db_blocks_map.keys().collect();

    let missing_in_db: HashSet<_> = file_keys.difference(&db_keys).collect();
    let extra_in_db: HashSet<_> = db_keys.difference(&file_keys).collect();

    // Print the symmetric difference
    if !missing_in_db.is_empty() || !extra_in_db.is_empty() {
        println!("Blocks missing in DB: {:?}", missing_in_db);
        println!("Blocks extra in DB: {:?}", extra_in_db);
    }
    assert_eq!(file_blocks_map.len(), db_blocks_map.len(), "Mismatch in number of blocks");

    // Check that all blocks have the correct canonical status
    let mut mismatches: Vec<((u64, String), bool, bool)> = vec![];
    for (key, &file_canonical) in &file_blocks_map {
        if let Some(&db_canonical) = db_blocks_map.get(key) {
            if file_canonical != db_canonical {
                mismatches.push((key.clone(), file_canonical, db_canonical));
            }
        } else {
            panic!("Block with height {} and state_hash {} not found in database", key.0, key.1);
        }
    }

    // Report mismatches
    if !mismatches.is_empty() {
        for ((height, state_hash), expected, actual) in &mismatches {
            println!(
                "Mismatch for height {} and state_hash {}: expected canonical {}, got {}",
                height, state_hash, expected, actual
            );
        }
    }

    assert!(mismatches.is_empty(), "Found mismatches between file and database canonical statuses");

    drop(handle);
}

async fn test_account_balances() {
    let file_content = std::fs::read_to_string(Path::new("./tests/data/ledger_at_height_5000.json")).expect("Failed to read JSON file from disk");

    // Parse the JSON into a vector of Account structs
    let accounts: Vec<Account> = sonic_rs::from_str(&file_content).unwrap();
    // let account_map: HashMap<String, Account> = accounts.into_iter().map(|account| (account.public_key.clone(), account)).collect();

    let (client, connection) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls)
        .await
        .expect("Expected to open conneciton");

    let handle = tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    let query = r#"
        SELECT address, CAST(balance AS BIGINT) AS balance
        FROM account_summary
        WHERE address_type = 'BlockchainAddress'
        ORDER BY balance ASC;
    "#;

    // Execute the query using the SQL client from the actor
    let rows = client
        .query(query, &[])
        .await
        .map_err(|_| "Unable to fetch account balances")
        .expect("Unable to execute query");

    let rows_map: HashMap<String, i64> = rows
        .into_iter()
        .map(|row| (row.get::<_, String>("address"), row.get::<_, i64>("balance")))
        .collect();

    let mut incorrect_accounts: Vec<(String, i64, i64)> = vec![];
    for account in accounts {
        assert!(rows_map.contains_key(&account.public_key));
        let ledger_account_balance = rows_map.get(&account.public_key).expect("Unable to get address from hash map");
        if &(account.balance as i64) != ledger_account_balance {
            incorrect_accounts.push((account.public_key.to_string(), account.balance as i64, ledger_account_balance.to_owned()));
            println!(
                "{}: {} != {} (diff: {})",
                account.public_key,
                account.balance,
                ledger_account_balance,
                account.balance as i64 - ledger_account_balance
            );
        }
    }

    assert_eq!(incorrect_accounts.len(), 0, "Expected ledger to match");

    drop(handle);
}
/// Spawns a child process for an integration test.
///
/// # Arguments
/// - `binary_path`: The path to the binary (e.g., `env!("CARGO_BIN_EXE_tool")`).
/// - `env_vars`: A list of environment variables to set for the process.
/// - `args`: A list of command-line arguments to pass to the binary.
/// - `timeout`: Duration for which the process is allowed to run.
pub fn run_test_process(binary_path: &str, env_vars: &[(&str, &str)], args: &[&str], timeout: Duration) {
    println!("Running ingestion process for {} minutes...", timeout.as_secs() / 60);

    // Spawn the child process with environment variables and arguments
    let mut child = {
        let mut cmd = Command::new(binary_path);
        let command = cmd
            .args(args) // Add command-line arguments
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        for (key, value) in env_vars {
            command.env(key, value);
        }

        command.spawn().expect("Failed to spawn child process")
    };

    // Ensure we have a stdout to read
    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let stderr = child.stderr.take().expect("Failed to capture stderr");

    // Create buffered readers for stdout and stderr
    let stdout_reader = BufReader::new(stdout);
    let stderr_reader = BufReader::new(stderr);

    // Spawn threads for real-time output streaming
    let stdout_thread = thread::spawn(move || {
        for line in stdout_reader.lines() {
            match line {
                Ok(line) => println!("STDOUT: {}", line),
                Err(err) => eprintln!("Error reading stdout: {}", err),
            }
        }
    });

    let stderr_thread = thread::spawn(move || {
        for line in stderr_reader.lines() {
            match line {
                Ok(line) => eprintln!("STDERR: {}", line),
                Err(err) => eprintln!("Error reading stderr: {}", err),
            }
        }
    });

    // Monitor the process and enforce the timeout
    let start = Instant::now();
    loop {
        if start.elapsed() >= timeout {
            // Kill the process if it exceeds the timeout
            child.kill().expect("Failed to kill the child process");
            println!("Process killed after timeout");
            break;
        }

        // Check if the process has exited
        if let Ok(Some(status)) = child.try_wait() {
            println!("Process exited with status: {}", status);
            break;
        }

        // Avoid busy-waiting
        thread::sleep(Duration::from_millis(100));
    }

    // Ensure the process is fully terminated
    child.wait().expect("Failed to wait on child process");

    // Wait for the streaming threads to finish
    stdout_thread.join().expect("Failed to join stdout thread");
    stderr_thread.join().expect("Failed to join stderr thread");
}
