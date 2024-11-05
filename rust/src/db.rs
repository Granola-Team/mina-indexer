use anyhow::Result;
use edgedb_protocol::query_arg::QueryArgs;
use edgedb_protocol::QueryResult;
use edgedb_tokio::{Builder, Client, RetryOptions};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

const MIN_CONNECTIONS: usize = 4;
pub const MAX_CONNECTIONS: usize = 32;
const CONNECTIONS_PER_CORE: usize = 4;

#[inline]
fn calculate_pool_size() -> usize {
    let cpu_count = num_cpus::get();
    let base_size = cpu_count * CONNECTIONS_PER_CORE;
    std::cmp::min(MAX_CONNECTIONS, std::cmp::max(MIN_CONNECTIONS, base_size))
}

/// Database pool
pub struct DbPool {
    client: Arc<Client>,
    active_connections: AtomicUsize,
    max_connections: usize,
}

impl DbPool {
    /// Create a new [DbPool]. If [branch] is [None], the branch will default to "main"
    pub async fn new(branch: Option<&str>) -> Result<Self, edgedb_tokio::Error> {
        let max_connections = calculate_pool_size();
        let client = Client::new(
            &Builder::new()
                .max_concurrency(max_connections)
                .branch(branch.unwrap_or("main"))?
                .build_env()
                .await?,
        )
        .with_retry_options(RetryOptions::default());

        Ok(Self {
            client: Arc::new(client),
            active_connections: AtomicUsize::new(0),
            max_connections,
        })
    }

    /// Execute a query and don't expect result.
    pub async fn execute<T>(
        &self,
        query: impl AsRef<str>,
        arguments: &T,
    ) -> Result<(), edgedb_tokio::Error>
    where
        T: QueryArgs + Send + Sync + 'static,
    {
        self.active_connections.fetch_add(1, Ordering::SeqCst);
        let result = self.client.execute(&query, arguments).await;
        self.active_connections.fetch_sub(1, Ordering::SeqCst);
        result
    }

    // Execute a query and return a collection of results.
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
