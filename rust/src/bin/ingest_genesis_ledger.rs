use env_logger::Builder;
use log::{error, info};
use mina_indexer::{
    constants::POSTGRES_CONNECTION_STRING,
    event_sourcing::{
        actors_v2::spawn_genesis_dag,
        events::{Event, EventType},
        payloads::{AccountingEntry, AccountingEntryAccountType, AccountingEntryType, DoubleEntryRecordPayload, GenesisBlockPayload, LedgerDestination},
        sourcing::get_genesis_ledger,
    },
};
use tokio::time::sleep;
use tokio_postgres::NoTls;

#[tokio::main]
async fn main() {
    // 1) Initialize logger
    Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .filter_module("tokio_postgres", log::LevelFilter::Warn)
        .init();

    let (client, connection) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls)
        .await
        .expect("Failed to connect to database");

    // Spawn the connection handle
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            error!("Postgres connection error: {}", e);
        }
    });

    if let Err(e) = client.execute("DROP SCHEMA public CASCADE;", &[]).await {
        eprintln!("unable to drop public schema: {}", e);
    }

    if let Err(e) = client.execute("CREATE SCHEMA public;", &[]).await {
        eprintln!("unable to public schema: {}", e);
    }

    let (dag, sender) = spawn_genesis_dag().await;

    for de in get_genesis_ledger().get_accounting_double_entries() {
        if let Err(err) = sender
            .send(Event {
                event_type: EventType::DoubleEntryTransaction,
                payload: sonic_rs::to_string(&de).unwrap(),
            })
            .await
        {
            error!("Failed to process a double entry from the genesis ledger: {err}");
        }
    }

    sleep(std::time::Duration::from_secs(1)).await;

    // Publish magic mina from genesis block
    let genesis_block = GenesisBlockPayload::new();
    let magic_mina = DoubleEntryRecordPayload {
        height: 1,
        state_hash: genesis_block.state_hash.to_string(),
        ledger_destination: LedgerDestination::BlockchainLedger,
        lhs: vec![AccountingEntry {
            counterparty: "MagicMinaForBlock0".to_string(),
            transfer_type: "BlockReward".to_string(),
            account: "B62qiy32p8kAKnny8ZFwoMhYpBppM1DWVCqAPBYNcXnsAHhnfAAuXgg".to_string(),
            entry_type: AccountingEntryType::Credit,
            account_type: AccountingEntryAccountType::BlockchainAddress,
            amount_nanomina: 1000,
            timestamp: genesis_block.unix_timestamp,
        }],
        rhs: vec![AccountingEntry {
            counterparty: "B62qiy32p8kAKnny8ZFwoMhYpBppM1DWVCqAPBYNcXnsAHhnfAAuXgg".to_string(),
            transfer_type: "BlockReward".to_string(),
            account: "MagicMinaForBlock0".to_string(),
            entry_type: AccountingEntryType::Debit,
            account_type: AccountingEntryAccountType::VirtualAddess,
            amount_nanomina: 1000,
            timestamp: genesis_block.unix_timestamp,
        }],
    };
    if let Err(err) = sender
        .send(Event {
            event_type: EventType::DoubleEntryTransaction,
            payload: sonic_rs::to_string(&magic_mina).unwrap(),
        })
        .await
    {
        error!("Failed to process a double entry (magic mina) from the genesis block: {err}");
    }

    sleep(std::time::Duration::from_secs(1)).await;

    for account in get_genesis_ledger().get_accounts() {
        if let Err(_err) = sender
            .send(Event {
                event_type: EventType::PreExistingAccount,
                payload: account,
            })
            .await
        {
            error!("Failed to process a pre-existing account");
        };
    }

    // 9) Give the DAG time to flush any in-flight operations
    info!("Giving some time for the DAG to flush...");
    dag.lock().await.wait_until_quiesced().await;

    info!("Shutting down gracefully. Goodbye!");
}
