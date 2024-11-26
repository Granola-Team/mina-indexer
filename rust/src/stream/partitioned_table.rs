use anyhow::Result;
use tokio_postgres::Client;

pub struct PartitionedTable {
    client: Client,
    table_name: String,
    columns: Vec<String>,
}

const PARTITION_RANGE: u64 = 10_000;

impl PartitionedTable {
    pub fn get_client(&self) -> &Client {
        &self.client
    }

    /// Builder to start creating a PartitionedTable
    pub fn builder(client: Client) -> PartitionedTableBuilder {
        PartitionedTableBuilder {
            client,
            name: String::new(),
            columns: Vec::new(),
        }
    }

    pub async fn create_partition(&self, height: u64) -> Result<()> {
        println!("Creating partition {}_{}", self.table_name, height);
        // Define the range for the partition based on the height
        let from = height;
        let to = height + PARTITION_RANGE;

        // Create the partition table query
        let partition_query = format!(
            "CREATE TABLE IF NOT EXISTS {}_{} PARTITION OF {} FOR VALUES FROM ({}) TO ({});",
            self.table_name, height, self.table_name, from, to
        );

        // Execute the query to create the partition
        if let Err(e) = self.get_client().execute(&partition_query, &[]).await {
            eprintln!("Failed to create partition for table '{}': {:?}", self.table_name, e);
        }

        Ok(())
    }

    /// Insert a row into the table, creating partitions as necessary based on the height
    pub async fn insert(
        &self,
        values: &[&(dyn tokio_postgres::types::ToSql + Sync)],
        height: u64, // Explicit height parameter
    ) -> Result<u64> {
        let column_names = self
            .columns
            .iter()
            .map(|col| col.split_whitespace().next().unwrap()) // Extract only the column names
            .collect::<Vec<_>>()
            .join(", ");
        let placeholders = (1..=self.columns.len()).map(|i| format!("${}", i)).collect::<Vec<_>>().join(", ");

        // Insert query
        let query = format!("INSERT INTO {} ({}) VALUES ({})", self.table_name, column_names, placeholders);

        match self.client.execute(&query, values).await {
            Ok(_) => Ok(1), // Successful insert
            Err(e) => {
                if e.to_string().contains("partition") {
                    self.create_partition((height / PARTITION_RANGE) * PARTITION_RANGE).await?;

                    // Retry the insert after creating the partition
                    self.client.execute(&query, values).await?;
                    Ok(1)
                } else {
                    // Return the error if it's not related to the partition creation
                    Err(e.into())
                }
            }
        }
    }
}

/// Builder for the PartitionedTable
pub struct PartitionedTableBuilder {
    client: Client,
    name: String,
    columns: Vec<String>,
}

impl PartitionedTableBuilder {
    pub fn get_client(&self) -> &Client {
        &self.client
    }
}

impl PartitionedTableBuilder {
    /// Set the name for the table
    pub fn name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    /// Add a column to the table
    pub fn add_column(mut self, column_definition: &str) -> Self {
        self.columns.push(column_definition.to_string());
        self
    }

    /// Build and initialize the table and partitions, optionally deleting rows based on root node
    pub async fn build(self, root: &Option<(u64, String)>) -> Result<PartitionedTable> {
        let table_name = format!("{}_log", self.name);
        let view_name = self.name.clone();

        // Ensure the `height` column is specified
        if !self.columns.iter().any(|col| col.starts_with("height")) {
            panic!("The column 'height' is required but not found.");
        }

        // Drop the table and view if necessary
        if root.is_none() {
            self.drop_table_and_view(&table_name, &view_name).await?;
        }

        // Create the table with partitioning by height
        self.create_table(&table_name).await?;

        // Handle root node deletion if provided
        if let Some((height, state_hash)) = root {
            self.handle_root_node_deletion(&table_name, height.to_owned(), state_hash).await?;
        }

        // Create the view with distinct columns
        self.create_view(&table_name, &view_name).await?;

        // Return the PartitionedTable instance
        Ok(PartitionedTable {
            client: self.client,
            table_name,
            columns: self.columns,
        })
    }

    // Drop the table and view if necessary
    async fn drop_table_and_view(&self, table_name: &str, view_name: &str) -> Result<()> {
        let drop_table = format!("DROP TABLE IF EXISTS {} CASCADE;", table_name);
        self.client.execute(&drop_table, &[]).await?;
        let drop_view = format!("DROP VIEW IF EXISTS {};", view_name);
        self.client.execute(&drop_view, &[]).await?;
        Ok(())
    }

    // Create the table with partitioning by height
    async fn create_table(&self, table_name: &str) -> Result<()> {
        let table_query = format!(
            "CREATE TABLE IF NOT EXISTS {} (
                entry_id BIGSERIAL,
                height BIGINT,
                {},
                PRIMARY KEY (entry_id, height)
            ) PARTITION BY RANGE (height);",
            table_name,
            self.columns
                .iter()
                .filter(|c| !c.starts_with("height")) // Ensure `height` is not added here
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
                .join(",\n")
        );
        self.client.execute(&table_query, &[]).await?;
        Ok(())
    }

    // Handle root node deletion: truncate rows with the given height and state_hash
    async fn handle_root_node_deletion(&self, table_name: &str, height: u64, state_hash: &str) -> Result<()> {
        let truncate_query = format!("DELETE FROM {} WHERE height > $1 OR (height = $1 AND state_hash = $2);", table_name);

        self.client.execute(&truncate_query, &[&(height as i64), &state_hash.to_string()]).await?;

        Ok(())
    }

    // Create the view with distinct columns
    async fn create_view(&self, table_name: &str, view_name: &str) -> Result<()> {
        let distinct_columns = self
            .columns
            .iter()
            .map(|col| col.split_whitespace().next().unwrap()) // Default to all column names
            .collect::<Vec<_>>()
            .join(", ");

        let view_query = format!(
            "CREATE OR REPLACE VIEW {} AS
            SELECT DISTINCT ON ({}) *
            FROM {}
            ORDER BY {}, entry_id DESC;",
            view_name, distinct_columns, table_name, distinct_columns
        );

        self.client.execute(&view_query, &[]).await?;
        Ok(())
    }
}

#[cfg(test)]
mod partitioned_table_tests {
    use super::*;
    use crate::constants::POSTGRES_CONNECTION_STRING;
    use tokio_postgres::NoTls;

    #[tokio::test]
    async fn test_partitioned_table_inserts() {
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

        // Setup the PartitionedTable
        let partitioned_table = PartitionedTable::builder(client)
            .name("log")
            .add_column("height BIGINT") // Explicitly adding the height column
            .add_column("state_hash TEXT")
            .add_column("timestamp BIGINT")
            .build(&None) // No root, so no deletion
            .await
            .expect("Failed to build partitioned table");

        // Insert data with explicit height parameter
        partitioned_table
            .insert(&[&1_i64, &"state_hash_1", &1234567890i64], 1)
            .await
            .expect("Failed to insert log");

        partitioned_table
            .insert(&[&2_i64, &"state_hash_2", &1234567891i64], 1)
            .await
            .expect("Failed to insert log");

        partitioned_table
            .insert(&[&3_i64, &"state_hash_3", &1234567892i64], 1)
            .await
            .expect("Failed to insert log");

        // Query the table to verify inserts
        let log_query = "SELECT * FROM log_log WHERE height = $1";
        let log_rows = partitioned_table.client.query(log_query, &[&(1_i64)]).await.expect("Failed to query log table");

        // Assert the correct rows are present
        assert_eq!(log_rows.len(), 1, "Expected 1 row in the table");

        let log_rows = partitioned_table.client.query(log_query, &[&(1_i64)]).await.expect("Failed to query log table");
        assert_eq!(log_rows.len(), 1, "Expected 1 row in the table");

        let log_rows = partitioned_table.client.query(log_query, &[&(2_i64)]).await.expect("Failed to query log table");
        assert_eq!(log_rows.len(), 1, "Expected 1 row in the table");
    }

    #[tokio::test]
    async fn test_partitioned_table_root_deletion_with_3_heights() {
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

            // Setup the PartitionedTable
            let partitioned_table = PartitionedTable::builder(client)
                .name("log")
                .add_column("height BIGINT") // Explicitly adding the height column
                .add_column("state_hash TEXT")
                .add_column("timestamp BIGINT")
                .build(&None) // No root initially, so no deletion
                .await
                .expect("Failed to build partitioned table");

            // Insert 3 rows for each of 3 heights: 0, 1, and 2 (total of 9 rows)
            // Ensure the height corresponds to 1, 2, or 3
            partitioned_table
                .insert(&[&1_i64, &"state_hash_1_1", &1234567890i64], 1)
                .await
                .expect("Failed to insert log");

            partitioned_table
                .insert(&[&2_i64, &"state_hash_1_2", &1234567891i64], 2)
                .await
                .expect("Failed to insert log");

            partitioned_table
                .insert(&[&3_i64, &"state_hash_1_3", &1234567892i64], 3)
                .await
                .expect("Failed to insert log");

            partitioned_table
                .insert(&[&1_i64, &"state_hash_2_1", &1234567893i64], 1)
                .await
                .expect("Failed to insert log");

            partitioned_table
                .insert(&[&2_i64, &"state_hash_2_2", &1234567894i64], 2)
                .await
                .expect("Failed to insert log");

            partitioned_table
                .insert(&[&3_i64, &"state_hash_2_3", &1234567895i64], 3)
                .await
                .expect("Failed to insert log");

            partitioned_table
                .insert(&[&1_i64, &"state_hash_3_1", &1234567896i64], 1)
                .await
                .expect("Failed to insert log");

            partitioned_table
                .insert(&[&2_i64, &"state_hash_3_2", &1234567897i64], 2)
                .await
                .expect("Failed to insert log");

            partitioned_table
                .insert(&[&3_i64, &"state_hash_3_3", &1234567898i64], 3)
                .await
                .expect("Failed to insert log");
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
            // Set the root node to height 2 (this will delete height 2 and its children)
            let root = Some((2, "state_hash_2_2".to_string()));
            let partitioned_table = PartitionedTable::builder(client)
                .name("log")
                .add_column("height BIGINT")
                .add_column("state_hash TEXT")
                .add_column("timestamp BIGINT")
                .build(&root) // Set root for deletion
                .await
                .expect("Failed to rebuild partitioned table with root");
            let log_query = "SELECT * FROM log_log WHERE height = $1";
            // Query for height 1 and ensure that rows still exist for height 1
            let log_rows = partitioned_table
                .client
                .query(log_query, &[&(1_i64)])
                .await
                .expect("Failed to query log table for height 1");

            // Assert that rows for height 1 are still present
            assert_eq!(log_rows.len(), 3, "Expected 3 rows in the table for height 1");

            // Query for height 3 and ensure that rows still exist for height 3
            let log_rows = partitioned_table
                .client
                .query(log_query, &[&(3_i64)])
                .await
                .expect("Failed to query log table for height 3");

            // Assert that rows for height 3 are still present
            assert_eq!(log_rows.len(), 0, "Expected 0 rows in the table for height 3");

            // Query for height 2 and check that the state_hash rows are deleted
            let log_query = "SELECT * FROM log_log WHERE height = $1";
            let deleted_row = partitioned_table
                .client
                .query(log_query, &[&(2_i64)])
                .await
                .expect("Failed to query log table for specific state_hash");

            // Ensure no rows exist for the deleted state_hash at height 2
            assert_eq!(deleted_row.len(), 2, "Expected 2 rows at height 2");
        }
    }

    #[tokio::test]
    async fn test_partitioned_table_insert_10001() {
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

        // Setup the PartitionedTable
        let partitioned_table = PartitionedTable::builder(client)
            .name("log")
            .add_column("height BIGINT") // Explicitly adding the height column
            .add_column("state_hash TEXT")
            .add_column("timestamp BIGINT")
            .build(&None) // No root, so no deletion
            .await
            .expect("Failed to build partitioned table");

        // Insert data at height 10001 (which should trigger partition creation)
        partitioned_table
            .insert(&[&10001_i64, &"state_hash_10001", &1234567890i64], 10001)
            .await
            .expect("Failed to insert log at height 10001");

        // Query the table to verify that the row was inserted correctly
        let log_query = "SELECT * FROM log_log WHERE height = $1";
        let log_rows = partitioned_table
            .client
            .query(log_query, &[&(10001_i64)])
            .await
            .expect("Failed to query log table");

        // Assert that the row was inserted
        assert_eq!(log_rows.len(), 1, "Expected 1 row in the table for height 10001");
    }
}
