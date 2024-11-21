use mina_indexer::constants::POSTGRES_CONNECTION_STRING;
use std::{
    io::{BufRead, BufReader},
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use tokio_postgres::NoTls;

#[tokio::test]
async fn test_blockchain_ledger() {
    run_test_process(
        env!("CARGO_BIN_EXE_ingestion"), // Binary path
        &[("BLOCKS_DIR", "./tests/data/5000_mainnet_blocks"), ("PUBLISH_RATE_PER_SECOND", "20")],
        Duration::from_secs(10 * 60), // 5-minute timeout
    );

    test_blockchain_ledger_accounting_per_block().await;
}

async fn test_blockchain_ledger_accounting_per_block() {
    if let Ok((client, connection)) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls).await {
        let handle = tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        let query = r#"
            SELECT SUM(balance_delta_sum) AS total_balance
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

/// Spawns a child process for an integration test.
///
/// # Arguments
/// - `binary_path`: The path to the binary (e.g., `env!("CARGO_BIN_EXE_tool")`).
/// - `env_vars`: A list of environment variables to set for the process.
/// - `timeout`: Duration for which the process is allowed to run.
pub fn run_test_process(binary_path: &str, env_vars: &[(&str, &str)], timeout: Duration) {
    // Spawn the child process with environment variables
    let mut child = {
        let mut cmd = Command::new(binary_path);
        let command = cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

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
