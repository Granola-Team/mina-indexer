use super::managed_table::ManagedTable;
use anyhow::Result;
use tokio_postgres::Client;

pub struct DbLogger {
    table: ManagedTable,
}

impl DbLogger {
    /// Builder to start creating a DbLogger
    pub fn builder(client: Client) -> DbLoggerBuilder {
        DbLoggerBuilder {
            client,
            name: String::new(),
            columns: Vec::new(),
            distinct_columns: Vec::new(),
        }
    }

    pub fn get_client(&self) -> &Client {
        self.table.get_client()
    }

    /// Insert a row into the table
    pub async fn insert(
        &self,
        values: &[&(dyn tokio_postgres::types::ToSql + Sync)],
        height: u64, // Explicit height parameter
    ) -> Result<u64> {
        self.table.insert(values, height).await
    }
}

/// Builder for the DbLogger
pub struct DbLoggerBuilder {
    client: Client,
    name: String,
    columns: Vec<String>,
    distinct_columns: Vec<String>,
}

impl DbLoggerBuilder {
    /// Set the name for the logger
    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    /// Add a column to the table
    pub fn add_column(mut self, column_definition: &str) -> Self {
        self.columns.push(column_definition.to_string());
        self
    }

    /// Specify the distinct columns for the view
    pub fn distinct_columns(mut self, columns: &[&str]) -> Self {
        self.distinct_columns = columns.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Build and initialize the table and view, dropping any existing table and view first
    pub async fn build(self, root_node: &Option<(u64, String)>) -> Result<DbLogger> {
        let table_name = format!("{}_log", self.name);
        let view_name = self.name.clone();

        let drop_view_query = format!("DROP VIEW IF EXISTS {};", view_name);
        self.client.execute(&drop_view_query, &[]).await?;

        let table_builder = self
            .columns
            .iter()
            .fold(ManagedTable::builder(self.client).name(&table_name), |builder, column| {
                builder.add_column(column)
            });

        let table = table_builder.build(root_node).await?;

        // Create the view with distinct columns
        let distinct_columns = if self.distinct_columns.is_empty() {
            self.columns
                .iter()
                .map(|col| col.split_whitespace().next().unwrap()) // Default to all column names
                .collect::<Vec<_>>()
                .join(", ")
        } else {
            self.distinct_columns.join(", ")
        };

        let view_query = format!(
            "CREATE OR REPLACE VIEW {} AS
                SELECT DISTINCT ON ({}) *
                FROM {}
                ORDER BY {}, entry_id DESC;",
            view_name, distinct_columns, table_name, distinct_columns
        );

        table.get_client().execute(&view_query, &[]).await?;

        Ok(DbLogger { table })
    }
}

#[cfg(test)]
mod db_logger_tests {
    use super::*;
    use crate::constants::POSTGRES_CONNECTION_STRING;
    use tokio_postgres::NoTls;

    #[tokio::test]
    async fn test_db_logger_inserts_and_view_distinct() {
        // Connect to the database
        let (client, connection) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls)
            .await
            .expect("Failed to connect to the database");

        // Spawn the connection handler
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Connection error: {}", e);
            }
        });

        // Setup the logger
        let logger = DbLogger::builder(client)
            .name("log")
            .add_column("height BIGINT") // Explicitly adding the height column
            .add_column("state_hash TEXT")
            .add_column("timestamp BIGINT")
            .distinct_columns(&["height", "state_hash"])
            .build(&None)
            .await
            .expect("Failed to build logger");

        // Insert data with explicit height parameter
        logger
            .insert(&[&1_i64, &"state_hash_1", &1234567890i64], 0)
            .await
            .expect("Failed to insert log");
        logger
            .insert(&[&1_i64, &"state_hash_1", &1234567891i64], 0)
            .await
            .expect("Failed to insert log");
        logger
            .insert(&[&1_i64, &"state_hash_2", &1234567892i64], 0)
            .await
            .expect("Failed to insert log");

        // Query the table
        let log_query = "SELECT * FROM log_log WHERE height = $1";
        let log_rows = logger.get_client().query(log_query, &[&(1_i64)]).await.expect("Failed to query log table");

        // Assert all rows are present in the table
        assert_eq!(log_rows.len(), 3, "Expected 3 rows in the table");

        // Query the view
        let view_query = "SELECT * FROM log WHERE height = $1";
        let view_rows = logger.get_client().query(view_query, &[&(1_i64)]).await.expect("Failed to query view");

        // Assert only the latest row for each state_hash is present in the view
        assert_eq!(view_rows.len(), 2, "Expected 2 rows in the view");
        let earliest_row: i64 = view_rows.iter().map(|row| row.get("timestamp")).min().unwrap();
        assert_eq!(earliest_row, 1234567891i64, "Expected the earliest timestamp in the view");
        let latest_row: i64 = view_rows.iter().map(|row| row.get("timestamp")).max().unwrap();
        assert_eq!(latest_row, 1234567892i64, "Expected the latest timestamp in the view");
    }

    #[tokio::test]
    async fn test_db_logger_with_sibling_nodes_and_children_rebuild() {
        let log_query = "SELECT * FROM log_log WHERE height = $1";
        let child_query = "SELECT * FROM log_log WHERE height = $1";
        {
            // Connect to the database
            let (client, connection) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls)
                .await
                .expect("Failed to connect to the database");

            // Spawn the connection handler
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    eprintln!("Connection error: {}", e);
                }
            });

            // Setup the logger
            let logger = DbLogger::builder(client)
                .name("log")
                .add_column("height BIGINT")
                .add_column("state_hash TEXT")
                .add_column("timestamp BIGINT")
                .distinct_columns(&["height", "state_hash"])
                .build(&None)
                .await
                .expect("Failed to build logger");

            // Insert three sibling nodes at height 2
            logger
                .insert(&[&2i64, &"state_hash_a", &1234567890i64], 0)
                .await
                .expect("Failed to insert sibling A");
            logger
                .insert(&[&2i64, &"state_hash_b", &1234567891i64], 0)
                .await
                .expect("Failed to insert sibling B");
            logger
                .insert(&[&2i64, &"state_hash_c", &1234567892i64], 0)
                .await
                .expect("Failed to insert sibling C");

            // Add children to state_hash_b
            logger
                .insert(&[&3i64, &"state_hash_b_child_1", &1234567893i64], 0)
                .await
                .expect("Failed to insert child 1 of state_hash_b");
            logger
                .insert(&[&3i64, &"state_hash_b_child_2", &1234567894i64], 0)
                .await
                .expect("Failed to insert child 2 of state_hash_b");

            // Verify all sibling nodes are in the table
            let log_rows = logger.get_client().query(log_query, &[&(2_i64)]).await.expect("Failed to query log table");
            assert_eq!(log_rows.len(), 3, "Expected 3 sibling rows in the table");

            // Verify children of state_hash_b are in the table
            let child_rows = logger
                .get_client()
                .query(child_query, &[&(3_i64)])
                .await
                .expect("Failed to query children of state_hash_b");
            assert_eq!(child_rows.len(), 2, "Expected 2 children of state_hash_b in the table");
        }
        {
            // Connect to the database
            let (client, connection) = tokio_postgres::connect(POSTGRES_CONNECTION_STRING, NoTls)
                .await
                .expect("Failed to connect to the database");

            // Spawn the connection handler
            tokio::spawn(async move {
                if let Err(e) = connection.await {
                    eprintln!("Connection error: {}", e);
                }
            });

            // Rebuild the logger with "state_hash_b" as the new root
            let new_root = (2u64, "state_hash_b".to_string());
            let logger = DbLogger::builder(client)
                .name("log")
                .add_column("height BIGINT")
                .add_column("state_hash TEXT")
                .add_column("timestamp BIGINT")
                .distinct_columns(&["height", "state_hash"])
                .build(&Some(new_root))
                .await
                .expect("Failed to rebuild logger");

            // Verify that "state_hash_b" (the root) and its children were deleted
            let log_rows = logger
                .get_client()
                .query(log_query, &[&(2_i64)])
                .await
                .expect("Failed to query log table after rebuild");

            // Extract remaining state_hash values
            let remaining_state_hashes: Vec<String> = log_rows.iter().map(|row| row.get("state_hash")).collect();

            // Ensure "state_hash_b" is not present, and the other siblings remain
            assert!(
                !remaining_state_hashes.contains(&"state_hash_b".to_string()),
                "state_hash_b (root) should have been deleted"
            );
            assert!(
                remaining_state_hashes.contains(&"state_hash_a".to_string()),
                "state_hash_a should remain in the table"
            );
            assert!(
                remaining_state_hashes.contains(&"state_hash_c".to_string()),
                "state_hash_c should remain in the table"
            );

            // Verify that only the expected sibling nodes are present
            assert_eq!(remaining_state_hashes.len(), 2, "Expected exactly 2 sibling rows remaining after root deletion");

            // Verify children of state_hash_b were deleted
            let child_rows = logger
                .get_client()
                .query(child_query, &[&(3_i64)])
                .await
                .expect("Failed to query children of state_hash_b after rebuild");
            assert!(child_rows.is_empty(), "Children of state_hash_b should have been deleted");
        }
    }
}
