use anyhow::Result;
use edgedb_protocol::query_arg::QueryArgs;
use edgedb_protocol::QueryResult;
use edgedb_tokio::{Builder, Client, RetryCondition, RetryOptions};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

const MIN_CONNECTIONS: usize = 4;
pub const MAX_CONNECTIONS: usize = 32;
const CONNECTIONS_PER_CORE: usize = 4;
const RETRY_ATTEMPTS: u32 = 5;

#[inline]
fn calculate_pool_size() -> usize {
    let cpu_count = num_cpus::get();
    let base_size = cpu_count * CONNECTIONS_PER_CORE;
    std::cmp::min(MAX_CONNECTIONS, std::cmp::max(MIN_CONNECTIONS, base_size))
}

pub struct DbPool {
    client: Arc<Client>,
    active_connections: AtomicUsize,
    max_connections: usize,
}

impl DbPool {
    pub async fn new(branch: Option<&str>) -> Result<Self, edgedb_tokio::Error> {
        let max_connections = calculate_pool_size();
        let client = Client::new(
            &Builder::new()
                .max_concurrency(max_connections)
                .branch(branch.unwrap_or("main"))?
                .build_env()
                .await?,
        )
        .with_retry_options(
            RetryOptions::default()
                .with_rule(
                    RetryCondition::TransactionConflict,
                    RETRY_ATTEMPTS,
                    |attempt| {
                        let base = Duration::from_millis(100);
                        let max_delay = Duration::from_secs(5);
                        let delay = base.mul_f64(1.5f64.powi(attempt as i32));
                        std::cmp::min(delay, max_delay)
                    },
                )
                .with_rule(RetryCondition::NetworkError, 3, |_| {
                    Duration::from_millis(500)
                }),
        );

        Ok(Self {
            client: Arc::new(client),
            active_connections: AtomicUsize::new(0),
            max_connections,
        })
    }

    pub async fn execute<T>(
        &self,
        query: impl AsRef<str>,
        arguments: T,
    ) -> Result<(), edgedb_tokio::Error>
    where
        T: QueryArgs + Send + Sync + 'static,
    {
        self.active_connections.fetch_add(1, Ordering::SeqCst);
        let result = self.client.execute(&query, &arguments).await;
        self.active_connections.fetch_sub(1, Ordering::SeqCst);
        result
    }

    pub async fn query<R, A>(
        &self,
        query: impl AsRef<str>,
        arguments: &A,
    ) -> Result<Vec<R>, edgedb_tokio::Error>
    where
        R: QueryResult,
        A: QueryArgs,
    {
        self.active_connections.fetch_add(1, Ordering::SeqCst);
        let result = self.client.query(&query, arguments).await;
        self.active_connections.fetch_sub(1, Ordering::SeqCst);
        result
    }

    pub fn get_pool_stats(&self) -> String {
        format!(
            ", DB connections: {}/{} active",
            self.active_connections.load(Ordering::Relaxed),
            self.max_connections
        )
    }
}
