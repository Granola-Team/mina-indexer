use anyhow::Result;
use edgedb_tokio::{Builder, Client, RetryCondition, RetryOptions};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

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
    inner: Arc<Mutex<Client>>,
    active_connections: AtomicUsize,
}

impl DbPool {
    pub async fn new() -> Result<Self, edgedb_tokio::Error> {
        let client = Client::new(
            &Builder::new()
                .max_concurrency(calculate_pool_size())
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
            inner: Arc::new(Mutex::new(client)),
            active_connections: AtomicUsize::new(0),
        })
    }

    pub async fn execute_internal<F, T>(&self, operation: F) -> Result<T, edgedb_tokio::Error>
    where
        F: FnOnce(&Client) -> futures::future::BoxFuture<'_, Result<T, edgedb_tokio::Error>>,
    {
        self.active_connections.fetch_add(1, Ordering::SeqCst);
        let client = self.inner.lock().await;
        let result = operation(&client).await;
        self.active_connections.fetch_sub(1, Ordering::SeqCst);
        result
    }

    pub async fn execute<T>(&self, query: String, params: T) -> Result<(), edgedb_tokio::Error>
    where
        T: edgedb_protocol::query_arg::QueryArgs + Send + Sync + 'static,
    {
        self.execute_internal(|client| {
            Box::pin(async move { client.execute(&query, &params).await })
        })
        .await
    }

    pub async fn get_pool_stats(&self) -> PoolStats {
        PoolStats {
            active_connections: self.active_connections.load(Ordering::Relaxed),
            available_permits: calculate_pool_size(),
            max_connections: calculate_pool_size(),
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct PoolStats {
    pub active_connections: usize,
    pub available_permits: usize,
    pub max_connections: usize,
}
